use crate::model::*;
use anyhow::Result;
use serde_yaml::{Mapping, Value};

pub fn generate_envoy_yaml(loaded: &crate::load::Loaded) -> Result<Value> {
    let mut root = Mapping::new();

    // --- admin ---
    root.insert(s("admin"), gen_admin(&loaded.admin, &loaded.access_log));

    // --- static_resources ---
    let mut static_resources = Mapping::new();
    static_resources.insert(
        s("listeners"),
        gen_listeners(
            &loaded.defaults,
            &loaded.access_log,
            &loaded.domains,
            &loaded.policies,
            &loaded.listeners,
        ),
    );
    static_resources.insert(s("clusters"), gen_clusters(&loaded.upstreams));

    root.insert(s("static_resources"), Value::Mapping(static_resources));
    Ok(Value::Mapping(root))
}

fn gen_admin(admin: &AdminSpec, log: &AccessLogSpec) -> Value {
    let mut m = Mapping::new();
    m.insert(s("access_log"), stdout_access_log(log));
    m.insert(s("address"), socket_addr("TCP", &admin.address, admin.port));
    Value::Mapping(m)
}

fn gen_listeners(
    defaults: &DefaultsSpec,
    log: &AccessLogSpec,
    domains: &[DomainSpec],
    policies: &PoliciesSpec,
    listeners_spec: &ListenersSpec,
) -> Value {
    // :80 HTTP -> defaults.http_default_upstream
    // :443 TLS inspector + SNI split:
    // - terminate for domains with mode terminate_https_443
    // - default passthrough -> defaults.tls_passthrough_upstream
    let mut listeners = vec![
        Value::Mapping(gen_http_80_listener(defaults, log)),
        Value::Mapping(gen_https_443_sni_listener(defaults, log, domains, policies)),
    ];

    for internal in &listeners_spec.internal_http_listeners {
        listeners.push(Value::Mapping(gen_internal_http_listener(
            defaults,
            log,
            internal,
        )));
    }

    Value::Sequence(listeners)
}

fn gen_http_80_listener(defaults: &DefaultsSpec, log: &AccessLogSpec) -> Mapping {
    let mut listener = Mapping::new();
    listener.insert(s("name"), s("http_listener"));
    listener.insert(s("address"), socket_addr("TCP", "0.0.0.0", 80));

    let hcm = http_connection_manager(
        "external_http",
        log,
        route_config_single_vhost(
            "external_http_route",
            "all_hosts",
            vec!["*"],
            vec![route_prefix_to_cluster(
                "/",
                &defaults.http_default_upstream,
                Some(defaults.route_timeout.clone()),
                None,
            )],
            None,
        ),
        vec![http_filter_router()],
        None,
    );

    let filter_chain = filter_chain_http(hcm);
    listener.insert(
        s("filter_chains"),
        Value::Sequence(vec![Value::Mapping(filter_chain)]),
    );
    listener
}

fn gen_https_443_sni_listener(
    defaults: &DefaultsSpec,
    log: &AccessLogSpec,
    domains: &[DomainSpec],
    policies: &PoliciesSpec,
) -> Mapping {
    let mut listener = Mapping::new();
    listener.insert(s("name"), s("https_sni_listener"));
    listener.insert(s("address"), socket_addr("TCP", "0.0.0.0", 443));

    // tls_inspector
    listener.insert(
        s("listener_filters"),
        Value::Sequence(vec![Value::Mapping({
            let mut lf = Mapping::new();
            lf.insert(s("name"), s("envoy.filters.listener.tls_inspector"));
            lf.insert(s("typed_config"), {
                let mut tc = Mapping::new();
                tc.insert(s("@type"), s("type.googleapis.com/envoy.extensions.filters.listener.tls_inspector.v3.TlsInspector"));
                Value::Mapping(tc)
            });
            lf
        })]),
    );

    // filter chains
    let mut filter_chains = Vec::new();

    // termination chains
    for d in domains.iter().filter(|d| d.mode == "terminate_https_443") {
        let tls = d.tls.as_ref().expect("validated: tls exists");

        let mut fc = Mapping::new();

        // match SNI name
        fc.insert(s("filter_chain_match"), {
            let mut m = Mapping::new();
            m.insert(s("server_names"), Value::Sequence(vec![s(&d.domain)]));
            Value::Mapping(m)
        });

        // transport_socket TLS
        fc.insert(s("transport_socket"), downstream_tls_socket(tls));

        // HCM routes for this domain
        let mut routes = Vec::new();
        let any_route_uses_rl = d.routes.iter().any(|r| {
            r.per_filter_config
                .as_ref()
                .and_then(|p| p.local_ratelimit.as_ref())
                .is_some()
        });

        for r in &d.routes {
            routes.push(route_from_spec(r, defaults, policies));
        }

        let rc = route_config_single_vhost(
            format!("{}_route", sanitize_name(&d.domain)),
            format!("{}_vhost", sanitize_name(&d.domain)),
            vec![d.domain.as_str()],
            routes,
            None,
        );

        // Filters:
        // - If any route references local_ratelimit policy, include local_ratelimit filter
        // - Include any extra filters configured for this domain
        // - Always include router
        let mut http_filters = Vec::new();
        if any_route_uses_rl {
            let stat_prefix = d
                .http_connection_manager
                .as_ref()
                .and_then(|hcm| hcm.local_ratelimit_stat_prefix.as_deref())
                .unwrap_or("default_local_ratelimit");
            http_filters.push(http_filter_local_ratelimit_default(stat_prefix));
        }
        if let Some(hcm) = d.http_connection_manager.as_ref() {
            for filter in &hcm.extra_http_filters {
                match filter {
                    HttpFilterSpec::GrpcWeb => http_filters.push(http_filter_grpc_web()),
                }
            }
        }
        http_filters.push(http_filter_router());

        let hcm = http_connection_manager_with_domain(
            &format!("{}_https", sanitize_name(&d.domain)),
            log,
            rc,
            http_filters,
            d.http_connection_manager.as_ref(),
            d.normalize_path,
            d.merge_slashes,
            d.aws_signing.as_ref(),
        );

        fc.insert(
            s("filters"),
            Value::Sequence(vec![Value::Mapping({
                let mut f = Mapping::new();
                f.insert(
                    s("name"),
                    s("envoy.filters.network.http_connection_manager"),
                );
                f.insert(s("typed_config"), hcm);
                f
            })]),
        );

        filter_chains.push(Value::Mapping(fc));
    }

    // default passthrough chain -> tcp_proxy to defaults.tls_passthrough_upstream
    filter_chains.push(Value::Mapping({
        let mut fc = Mapping::new();
        fc.insert(
            s("filters"),
            Value::Sequence(vec![Value::Mapping(tcp_proxy_filter(
                "external_tls_passthrough",
                &defaults.tls_passthrough_upstream,
            ))]),
        );
        fc
    }));

    listener.insert(s("filter_chains"), Value::Sequence(filter_chains));
    listener
}

fn gen_internal_http_listener(
    defaults: &DefaultsSpec,
    log: &AccessLogSpec,
    internal: &InternalHttpListenerSpec,
) -> Mapping {
    let mut listener = Mapping::new();
    listener.insert(s("name"), s(&internal.name));
    listener.insert(
        s("address"),
        socket_addr("TCP", &internal.address, internal.port),
    );

    let timeout = internal
        .timeout
        .clone()
        .or(Some(defaults.route_timeout.clone()));
    let routes = vec![route_prefix_to_cluster(
        "/",
        &internal.to_upstream,
        timeout,
        None,
    )];

    let rc = route_config_single_vhost(
        format!("{}_route", internal.name),
        format!("{}_vhost", internal.name),
        internal.domains.iter().map(String::as_str).collect(),
        routes,
        Some(&internal.request_headers_to_add),
    );

    let hcm = http_connection_manager(
        &internal.stat_prefix,
        log,
        rc,
        vec![http_filter_router()],
        None,
    );

    let filter_chain = filter_chain_http(hcm);
    listener.insert(
        s("filter_chains"),
        Value::Sequence(vec![Value::Mapping(filter_chain)]),
    );
    listener
}

fn route_from_spec(r: &RouteSpec, defaults: &DefaultsSpec, policies: &PoliciesSpec) -> Value {
    let mut route = Mapping::new();
    route.insert(s("match"), match_to_value(&r.m));

    // Check if this is a direct response route
    if let Some(dr) = &r.direct_response {
        let mut direct = Mapping::new();
        direct.insert(s("status"), n(dr.status as u64));
        if let Some(body) = &dr.body {
            direct.insert(
                s("body"),
                Value::Mapping({
                    let mut b = Mapping::new();
                    b.insert(s("inline_string"), s(body));
                    b
                }),
            );
        }
        route.insert(s("direct_response"), Value::Mapping(direct));
        return Value::Mapping(route);
    }

    // Regular route action
    let timeout = r.timeout.clone().or(Some(defaults.route_timeout.clone()));

    let mut route_action = Mapping::new();
    if let Some(upstream) = &r.to_upstream {
        route_action.insert(s("cluster"), s(upstream));
    }
    if let Some(t) = timeout {
        route_action.insert(s("timeout"), s(t));
    }
    if let Some(rewrite) = &r.prefix_rewrite {
        route_action.insert(s("prefix_rewrite"), s(rewrite));
    }
    route.insert(s("route"), Value::Mapping(route_action));

    // per-route typed_per_filter_config (local_ratelimit)
    if let Some(pfc) = &r.per_filter_config {
        if let Some(key) = &pfc.local_ratelimit {
            let tb = policies.local_ratelimits.get(key).expect("validated");
            let mut typed = Mapping::new();
            let stat_prefix = tb
                .stat_prefix
                .as_deref()
                .map(str::to_string)
                .unwrap_or_else(|| format!("rl_{}", key));
            typed.insert(
                s("envoy.filters.http.local_ratelimit"),
                Value::Mapping({
                    let mut cfg = Mapping::new();
                    cfg.insert(s("@type"), s("type.googleapis.com/envoy.extensions.filters.http.local_ratelimit.v3.LocalRateLimit"));
                    cfg.insert(s("stat_prefix"), s(stat_prefix));
                    cfg.insert(s("token_bucket"), Value::Mapping({
                        let mut t = Mapping::new();
                        t.insert(s("max_tokens"), n(tb.max_tokens));
                        t.insert(s("tokens_per_fill"), n(tb.tokens_per_fill));
                        t.insert(s("fill_interval"), s(&tb.fill_interval));
                        t
                    }));
                    cfg
                }),
            );
            route.insert(s("typed_per_filter_config"), Value::Mapping(typed));
        }
    }

    Value::Mapping(route)
}

fn match_to_value(m: &MatchSpec) -> Value {
    let mut mm = Mapping::new();
    if let Some(prefix) = &m.prefix {
        mm.insert(s("prefix"), s(prefix));
    } else if let Some(path) = &m.path {
        mm.insert(s("path"), s(path));
    }

    // Add header matchers if present
    if !m.headers.is_empty() {
        let headers: Vec<Value> = m
            .headers
            .iter()
            .map(|h| {
                let mut hm = Mapping::new();
                hm.insert(s("name"), s(&h.name));
                if let Some(exact) = &h.exact_match {
                    hm.insert(s("exact_match"), s(exact));
                }
                Value::Mapping(hm)
            })
            .collect();
        mm.insert(s("headers"), Value::Sequence(headers));
    }

    Value::Mapping(mm)
}

/* ---------------- clusters ---------------- */

fn gen_clusters(upstreams: &[UpstreamSpec]) -> Value {
    let mut ups: Vec<_> = upstreams.iter().collect();
    ups.sort_by(|a, b| a.name.cmp(&b.name));

    Value::Sequence(
        ups.into_iter()
            .map(|u| Value::Mapping(gen_cluster(u)))
            .collect(),
    )
}

fn gen_cluster(u: &UpstreamSpec) -> Mapping {
    let mut m = Mapping::new();
    m.insert(s("name"), s(&u.name));
    m.insert(s("connect_timeout"), s(&u.connect_timeout));
    m.insert(s("type"), s(&u.r#type));
    m.insert(s("lb_policy"), s(&u.lb_policy));

    if u.http2 {
        m.insert(s("http2_protocol_options"), Value::Mapping(Mapping::new()));
    }

    let mut load_assignment = Mapping::new();
    load_assignment.insert(s("cluster_name"), s(&u.name));

    let mut lb_eps = Vec::new();
    for ep in &u.endpoints {
        let mut e = Mapping::new();
        e.insert(
            s("endpoint"),
            Value::Mapping({
                let mut endpoint = Mapping::new();
                endpoint.insert(s("address"), socket_addr("TCP", &ep.address, ep.port));
                endpoint
            }),
        );
        lb_eps.push(Value::Mapping(e));
    }

    load_assignment.insert(
        s("endpoints"),
        Value::Sequence(vec![Value::Mapping({
            let mut e = Mapping::new();
            e.insert(s("lb_endpoints"), Value::Sequence(lb_eps));
            e
        })]),
    );

    m.insert(s("load_assignment"), Value::Mapping(load_assignment));
    m
}

/* ---------------- building blocks ---------------- */

fn http_connection_manager(
    stat_prefix: &str,
    log: &AccessLogSpec,
    route_config: Value,
    http_filters: Vec<Value>,
    overrides: Option<&HttpConnectionManagerSpec>,
) -> Value {
    http_connection_manager_with_domain(stat_prefix, log, route_config, http_filters, overrides, None, None, None)
}

#[allow(clippy::too_many_arguments)]
fn http_connection_manager_with_domain(
    stat_prefix: &str,
    log: &AccessLogSpec,
    route_config: Value,
    http_filters: Vec<Value>,
    overrides: Option<&HttpConnectionManagerSpec>,
    domain_normalize_path: Option<bool>,
    domain_merge_slashes: Option<bool>,
    aws_signing: Option<&AwsSigningSpec>,
) -> Value {
    let mut hcm = Mapping::new();
    let stat_prefix = overrides
        .and_then(|o| o.stat_prefix.as_deref())
        .unwrap_or(stat_prefix);
    hcm.insert(s("@type"), s("type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager"));
    hcm.insert(s("stat_prefix"), s(stat_prefix));

    // normalize_path: domain-level takes precedence, then HCM override, then default true
    let normalize = domain_normalize_path
        .or_else(|| overrides.and_then(|o| o.normalize_path))
        .unwrap_or(true);
    hcm.insert(s("normalize_path"), b(normalize));

    // merge_slashes: domain-level takes precedence, then HCM override, then default true
    let merge = domain_merge_slashes
        .or_else(|| overrides.and_then(|o| o.merge_slashes))
        .unwrap_or(true);
    hcm.insert(s("merge_slashes"), b(merge));

    if let Some(use_remote_address) = overrides.and_then(|o| o.use_remote_address) {
        hcm.insert(s("use_remote_address"), b(use_remote_address));
    }
    if let Some(xff_num_trusted_hops) = overrides.and_then(|o| o.xff_num_trusted_hops) {
        hcm.insert(s("xff_num_trusted_hops"), n(xff_num_trusted_hops));
    }
    if let Some(stream_idle_timeout) = overrides.and_then(|o| o.stream_idle_timeout.as_deref())
    {
        hcm.insert(s("stream_idle_timeout"), s(stream_idle_timeout));
    }
    hcm.insert(s("access_log"), stdout_access_log(log));
    hcm.insert(s("route_config"), route_config);

    // If AWS signing is enabled, we need to use upstream_http_filters in the router
    if let Some(signing) = aws_signing {
        if signing.enabled {
            // Build http_filters with upstream signing in the router
            let mut final_filters = Vec::new();
            for filter in http_filters {
                // Check if this is the router filter and add upstream signing
                if let Value::Mapping(ref m) = filter {
                    if m.get(Value::String("name".to_string())) == Some(&Value::String("envoy.filters.http.router".to_string())) {
                        final_filters.push(http_filter_router_with_aws_signing(signing));
                        continue;
                    }
                }
                final_filters.push(filter);
            }
            hcm.insert(s("http_filters"), Value::Sequence(final_filters));
        } else {
            hcm.insert(s("http_filters"), Value::Sequence(http_filters));
        }
    } else {
        hcm.insert(s("http_filters"), Value::Sequence(http_filters));
    }

    Value::Mapping(hcm)
}

fn route_config_single_vhost(
    name: impl Into<String>,
    vhost_name: impl Into<String>,
    domains: Vec<&str>,
    routes: Vec<Value>,
    request_headers_to_add: Option<&[HeaderValueOption]>,
) -> Value {
    let mut rc = Mapping::new();
    rc.insert(s("name"), s(name.into()));
    rc.insert(
        s("virtual_hosts"),
        Value::Sequence(vec![Value::Mapping({
            let mut vh = Mapping::new();
            vh.insert(s("name"), s(vhost_name.into()));
            vh.insert(
                s("domains"),
                Value::Sequence(domains.into_iter().map(s).collect()),
            );
            if let Some(headers) = request_headers_to_add {
                if !headers.is_empty() {
                    vh.insert(
                        s("request_headers_to_add"),
                        Value::Sequence(headers.iter().map(header_value_option).collect()),
                    );
                }
            }
            vh.insert(s("routes"), Value::Sequence(routes));
            vh
        })]),
    );
    Value::Mapping(rc)
}

fn route_prefix_to_cluster(
    prefix: &str,
    cluster: &str,
    timeout: Option<String>,
    _unused: Option<()>,
) -> Value {
    let mut route = Mapping::new();
    route.insert(
        s("match"),
        Value::Mapping({
            let mut m = Mapping::new();
            m.insert(s("prefix"), s(prefix));
            m
        }),
    );

    route.insert(
        s("route"),
        Value::Mapping({
            let mut r = Mapping::new();
            r.insert(s("cluster"), s(cluster));
            if let Some(t) = timeout {
                r.insert(s("timeout"), s(t));
            }
            r
        }),
    );

    Value::Mapping(route)
}

fn http_filter_router() -> Value {
    Value::Mapping({
        let mut f = Mapping::new();
        f.insert(s("name"), s("envoy.filters.http.router"));
        f.insert(
            s("typed_config"),
            Value::Mapping({
                let mut tc = Mapping::new();
                tc.insert(
                    s("@type"),
                    s("type.googleapis.com/envoy.extensions.filters.http.router.v3.Router"),
                );
                tc
            }),
        );
        f
    })
}

/// Router filter with AWS request signing in upstream_http_filters
/// This is used for S3-compatible backends where Envoy signs requests on behalf of anonymous clients
fn http_filter_router_with_aws_signing(signing: &AwsSigningSpec) -> Value {
    Value::Mapping({
        let mut f = Mapping::new();
        f.insert(s("name"), s("envoy.filters.http.router"));
        f.insert(
            s("typed_config"),
            Value::Mapping({
                let mut tc = Mapping::new();
                tc.insert(
                    s("@type"),
                    s("type.googleapis.com/envoy.extensions.filters.http.router.v3.Router"),
                );
                // upstream_http_filters: signing happens after Envoy finalizes the request
                tc.insert(
                    s("upstream_http_filters"),
                    Value::Sequence(vec![
                        // AWS Request Signing filter
                        Value::Mapping({
                            let mut sf = Mapping::new();
                            sf.insert(s("name"), s("envoy.filters.http.aws_request_signing"));
                            sf.insert(
                                s("typed_config"),
                                Value::Mapping({
                                    let mut stc = Mapping::new();
                                    stc.insert(
                                        s("@type"),
                                        s("type.googleapis.com/envoy.extensions.filters.http.aws_request_signing.v3.AwsRequestSigning"),
                                    );
                                    stc.insert(s("service_name"), s(&signing.service_name));
                                    stc.insert(s("region"), s(&signing.region));
                                    stc.insert(s("use_unsigned_payload"), b(signing.use_unsigned_payload));

                                    // Add credential_provider to use only environment variables
                                    // This prevents Envoy from trying IMDS/instance profile
                                    if signing.use_env_credentials {
                                        stc.insert(
                                            s("credential_provider"),
                                            Value::Mapping({
                                                let mut cp = Mapping::new();
                                                cp.insert(s("custom_credential_provider_chain"), b(true));
                                                cp.insert(s("environment_credential_provider"), Value::Mapping(Mapping::new()));
                                                cp
                                            }),
                                        );
                                    }
                                    stc
                                }),
                            );
                            sf
                        }),
                        // Upstream codec filter (required after signing)
                        Value::Mapping({
                            let mut uf = Mapping::new();
                            uf.insert(s("name"), s("envoy.filters.http.upstream_codec"));
                            uf.insert(
                                s("typed_config"),
                                Value::Mapping({
                                    let mut utc = Mapping::new();
                                    utc.insert(
                                        s("@type"),
                                        s("type.googleapis.com/envoy.extensions.filters.http.upstream_codec.v3.UpstreamCodec"),
                                    );
                                    utc
                                }),
                            );
                            uf
                        }),
                    ]),
                );
                tc
            }),
        );
        f
    })
}

fn http_filter_local_ratelimit_default(stat_prefix: &str) -> Value {
    Value::Mapping({
        let mut f = Mapping::new();
        f.insert(s("name"), s("envoy.filters.http.local_ratelimit"));
        f.insert(s("typed_config"), Value::Mapping({
            let mut tc = Mapping::new();
            tc.insert(s("@type"), s("type.googleapis.com/envoy.extensions.filters.http.local_ratelimit.v3.LocalRateLimit"));
            tc.insert(s("stat_prefix"), s(stat_prefix));
            tc
        }));
        f
    })
}

fn http_filter_grpc_web() -> Value {
    Value::Mapping({
        let mut f = Mapping::new();
        f.insert(s("name"), s("envoy.filters.http.grpc_web"));
        f.insert(
            s("typed_config"),
            Value::Mapping({
                let mut tc = Mapping::new();
                tc.insert(
                    s("@type"),
                    s("type.googleapis.com/envoy.extensions.filters.http.grpc_web.v3.GrpcWeb"),
                );
                tc
            }),
        );
        f
    })
}

fn header_value_option(h: &HeaderValueOption) -> Value {
    Value::Mapping({
        let mut m = Mapping::new();
        m.insert(
            s("header"),
            Value::Mapping({
                let mut hv = Mapping::new();
                hv.insert(s("key"), s(&h.header.key));
                hv.insert(s("value"), s(&h.header.value));
                hv
            }),
        );
        if let Some(append_action) = &h.append_action {
            m.insert(s("append_action"), s(append_action));
        }
        m
    })
}

fn tcp_proxy_filter(stat_prefix: &str, cluster: &str) -> Mapping {
    let mut f = Mapping::new();
    f.insert(s("name"), s("envoy.filters.network.tcp_proxy"));
    f.insert(
        s("typed_config"),
        Value::Mapping({
            let mut tc = Mapping::new();
            tc.insert(
                s("@type"),
                s("type.googleapis.com/envoy.extensions.filters.network.tcp_proxy.v3.TcpProxy"),
            );
            tc.insert(s("stat_prefix"), s(stat_prefix));
            tc.insert(s("cluster"), s(cluster));
            tc
        }),
    );
    f
}

fn filter_chain_http(hcm_typed_config: Value) -> Mapping {
    let mut fc = Mapping::new();
    fc.insert(
        s("filters"),
        Value::Sequence(vec![Value::Mapping({
            let mut f = Mapping::new();
            f.insert(
                s("name"),
                s("envoy.filters.network.http_connection_manager"),
            );
            f.insert(s("typed_config"), hcm_typed_config);
            f
        })]),
    );
    fc
}

fn downstream_tls_socket(tls: &TlsSpec) -> Value {
    Value::Mapping({
        let mut ts = Mapping::new();
        ts.insert(s("name"), s("envoy.transport_sockets.tls"));
        ts.insert(s("typed_config"), Value::Mapping({
            let mut tc = Mapping::new();
            tc.insert(s("@type"), s("type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.DownstreamTlsContext"));
            tc.insert(s("common_tls_context"), Value::Mapping({
                let mut ctc = Mapping::new();
                ctc.insert(s("tls_certificates"), Value::Sequence(vec![Value::Mapping({
                    let mut cert = Mapping::new();
                    cert.insert(s("certificate_chain"), Value::Mapping({
                        let mut cc = Mapping::new();
                        cc.insert(s("filename"), s(&tls.cert_chain));
                        cc
                    }));
                    cert.insert(s("private_key"), Value::Mapping({
                        let mut pk = Mapping::new();
                        pk.insert(s("filename"), s(&tls.private_key));
                        pk
                    }));
                    cert
                })]));
                ctc
            }));
            tc
        }));
        ts
    })
}

fn socket_addr(protocol: &str, address: &str, port: u16) -> Value {
    Value::Mapping({
        let mut a = Mapping::new();
        a.insert(
            s("socket_address"),
            Value::Mapping({
                let mut sa = Mapping::new();
                sa.insert(s("protocol"), s(protocol));
                sa.insert(s("address"), s(address));
                sa.insert(s("port_value"), n(port));
                sa
            }),
        );
        a
    })
}

fn stdout_access_log(log: &AccessLogSpec) -> Value {
    // Initial draft supports stdout file logger
    let mut entry = Mapping::new();
    entry.insert(s("name"), s("envoy.access_loggers.file"));
    entry.insert(
        s("typed_config"),
        Value::Mapping({
            let mut tc = Mapping::new();
            tc.insert(
                s("@type"),
                s("type.googleapis.com/envoy.extensions.access_loggers.file.v3.FileAccessLog"),
            );
            tc.insert(s("path"), s(&log.path));
            tc
        }),
    );
    Value::Sequence(vec![Value::Mapping(entry)])
}

fn sanitize_name(domain: &str) -> String {
    domain
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/* helpers */
fn s<T: Into<String>>(x: T) -> Value {
    Value::String(x.into())
}
fn n<T: Into<u64>>(x: T) -> Value {
    Value::Number(serde_yaml::Number::from(x.into()))
}
fn b(x: bool) -> Value {
    Value::Bool(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("example.com"), "example_com");
        assert_eq!(
            sanitize_name("api-test.example.com"),
            "api_test_example_com"
        );
        assert_eq!(sanitize_name("simple"), "simple");
        assert_eq!(sanitize_name("123.456.789"), "123_456_789");
    }

    #[test]
    fn test_socket_addr() {
        let result = socket_addr("TCP", "127.0.0.1", 8080);
        match &result {
            Value::Mapping(m) => {
                let socket_addr = m.get(&Value::String("socket_address".to_string())).unwrap();
                match socket_addr {
                    Value::Mapping(sa) => {
                        assert_eq!(
                            sa.get(&Value::String("protocol".to_string())).unwrap(),
                            &Value::String("TCP".to_string())
                        );
                        assert_eq!(
                            sa.get(&Value::String("address".to_string())).unwrap(),
                            &Value::String("127.0.0.1".to_string())
                        );
                        assert_eq!(
                            sa.get(&Value::String("port_value".to_string())).unwrap(),
                            &Value::Number(8080.into())
                        );
                    }
                    _ => panic!("Expected mapping for socket_address"),
                }
            }
            _ => panic!("Expected mapping for address"),
        }
    }

    #[test]
    fn test_generate_envoy_yaml_basic() {
        // Create a minimal loaded structure
        let loaded = crate::load::Loaded {
            admin: AdminSpec {
                address: "0.0.0.0".to_string(),
                port: 9901,
            },
            defaults: DefaultsSpec {
                route_timeout: "60s".to_string(),
                http_default_upstream: "default_http".to_string(),
                tls_passthrough_upstream: "default_tls".to_string(),
            },
            access_log: AccessLogSpec {
                r#type: "stdout".to_string(),
                path: "/dev/stdout".to_string(),
            },
            validate: ValidateSpec::Native {
                user: "envoy".to_string(),
                bin: "envoy".to_string(),
                config_path: "/etc/envoy/envoy.yaml".to_string(),
            },
            listeners: ListenersSpec::default(),
            domains: vec![],
            upstreams: vec![
                UpstreamSpec {
                    name: "default_http".to_string(),
                    connect_timeout: "5s".to_string(),
                    r#type: "STATIC".to_string(),
                    lb_policy: "ROUND_ROBIN".to_string(),
                    http2: false,
                    endpoints: vec![Endpoint {
                        address: "127.0.0.1".to_string(),
                        port: 8080,
                    }],
                },
                UpstreamSpec {
                    name: "default_tls".to_string(),
                    connect_timeout: "5s".to_string(),
                    r#type: "STATIC".to_string(),
                    lb_policy: "ROUND_ROBIN".to_string(),
                    http2: false,
                    endpoints: vec![Endpoint {
                        address: "127.0.0.1".to_string(),
                        port: 8443,
                    }],
                },
            ],
            policies: PoliciesSpec {
                local_ratelimits: Default::default(),
            },
        };

        let result = generate_envoy_yaml(&loaded);
        assert!(result.is_ok());

        let yaml_value = result.unwrap();
        match &yaml_value {
            Value::Mapping(m) => {
                // Check that we have the expected top-level keys
                assert!(m.contains_key(&Value::String("admin".to_string())));
                assert!(m.contains_key(&Value::String("static_resources".to_string())));
            }
            _ => panic!("Expected mapping for root"),
        }
    }

    #[test]
    fn test_generate_envoy_yaml_with_domain() {
        // Create a loaded structure with a domain
        let loaded = crate::load::Loaded {
            admin: AdminSpec {
                address: "0.0.0.0".to_string(),
                port: 9901,
            },
            defaults: DefaultsSpec {
                route_timeout: "60s".to_string(),
                http_default_upstream: "default_http".to_string(),
                tls_passthrough_upstream: "default_tls".to_string(),
            },
            access_log: AccessLogSpec {
                r#type: "stdout".to_string(),
                path: "/dev/stdout".to_string(),
            },
            validate: ValidateSpec::Native {
                user: "envoy".to_string(),
                bin: "envoy".to_string(),
                config_path: "/etc/envoy/envoy.yaml".to_string(),
            },
            listeners: ListenersSpec::default(),
            domains: vec![DomainSpec {
                domain: "example.com".to_string(),
                mode: "terminate_https_443".to_string(),
                tls: Some(TlsSpec {
                    cert_chain: "/path/to/cert".to_string(),
                    private_key: "/path/to/key".to_string(),
                }),
                routes: vec![RouteSpec {
                    m: MatchSpec {
                        prefix: Some("/api".to_string()),
                        path: None,
                        headers: vec![],
                    },
                    to_upstream: Some("api_backend".to_string()),
                    timeout: Some("30s".to_string()),
                    per_filter_config: None,
                    prefix_rewrite: None,
                    direct_response: None,
                }],
                http_connection_manager: None,
                normalize_path: None,
                merge_slashes: None,
                aws_signing: None,
            }],
            upstreams: vec![
                UpstreamSpec {
                    name: "api_backend".to_string(),
                    connect_timeout: "5s".to_string(),
                    r#type: "STATIC".to_string(),
                    lb_policy: "ROUND_ROBIN".to_string(),
                    http2: false,
                    endpoints: vec![Endpoint {
                        address: "127.0.0.1".to_string(),
                        port: 8080,
                    }],
                },
                UpstreamSpec {
                    name: "default_http".to_string(),
                    connect_timeout: "5s".to_string(),
                    r#type: "STATIC".to_string(),
                    lb_policy: "ROUND_ROBIN".to_string(),
                    http2: false,
                    endpoints: vec![Endpoint {
                        address: "127.0.0.1".to_string(),
                        port: 8080,
                    }],
                },
                UpstreamSpec {
                    name: "default_tls".to_string(),
                    connect_timeout: "5s".to_string(),
                    r#type: "STATIC".to_string(),
                    lb_policy: "ROUND_ROBIN".to_string(),
                    http2: false,
                    endpoints: vec![Endpoint {
                        address: "127.0.0.1".to_string(),
                        port: 8443,
                    }],
                },
            ],
            policies: PoliciesSpec {
                local_ratelimits: Default::default(),
            },
        };

        let result = generate_envoy_yaml(&loaded);
        assert!(result.is_ok());

        let yaml_value = result.unwrap();
        match &yaml_value {
            Value::Mapping(m) => {
                // Check that we have the expected top-level keys
                assert!(m.contains_key(&Value::String("admin".to_string())));
                assert!(m.contains_key(&Value::String("static_resources".to_string())));

                // Check static_resources structure
                let static_resources = m
                    .get(&Value::String("static_resources".to_string()))
                    .unwrap();
                match static_resources {
                    Value::Mapping(sr) => {
                        assert!(sr.contains_key(&Value::String("listeners".to_string())));
                        assert!(sr.contains_key(&Value::String("clusters".to_string())));
                    }
                    _ => panic!("Expected mapping for static_resources"),
                }
            }
            _ => panic!("Expected mapping for root"),
        }
    }

    #[test]
    fn test_match_to_value() {
        // Test prefix match
        let prefix_match = MatchSpec {
            prefix: Some("/api".to_string()),
            path: None,
            headers: vec![],
        };
        let result = match_to_value(&prefix_match);
        match &result {
            Value::Mapping(m) => {
                assert_eq!(
                    m.get(&Value::String("prefix".to_string())).unwrap(),
                    &Value::String("/api".to_string())
                );
            }
            _ => panic!("Expected mapping for match"),
        }

        // Test path match
        let path_match = MatchSpec {
            prefix: None,
            path: Some("/exact/path".to_string()),
            headers: vec![],
        };
        let result = match_to_value(&path_match);
        match &result {
            Value::Mapping(m) => {
                assert_eq!(
                    m.get(&Value::String("path".to_string())).unwrap(),
                    &Value::String("/exact/path".to_string())
                );
            }
            _ => panic!("Expected mapping for match"),
        }
    }
}
