use std::collections::HashMap;

use opentelemetry::{
    propagation::TextMapPropagator,
    sdk::{
        self,
        propagation::{BaggagePropagator, TextMapCompositePropagator, TraceContextPropagator},
        trace::Tracer,
        Resource,
    },
    trace::TraceError,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_semantic_conventions as semconv;
use tracing::{info, Subscriber};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, registry::LookupSpan, EnvFilter, Layer,
};

pub fn build_logger_text<S>() -> Box<dyn Layer<S> + Send + Sync + 'static>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    if cfg!(debug_assertions) {
        Box::new(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_line_number(true)
                .with_thread_names(true)
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::uptime()),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .with_span_events(FmtSpan::NEW | FmtSpan::ENTER | FmtSpan::EXIT | FmtSpan::CLOSE)
                .with_timer(tracing_subscriber::fmt::time::uptime()),
        )
    }
}

pub fn build_loglevel_filter_layer() -> EnvFilter {
    // filter what is output on log (fmt)
    // std::env::set_var("RUST_LOG", "warn,otel::tracing=info,otel=debug");
    std::env::set_var(
        "RUST_LOG",
        format!(
            // `otel::tracing` should be a level info to emit opentelemetry trace & span
            // `otel::setup` set to debug to log detected resources, configuration read and infered
            "{},otel::tracing=trace,otel=debug",
            std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("OTEL_LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string())
        ),
    );
    EnvFilter::from_default_env()
}

#[allow(clippy::box_default)]
fn propagator_from_string(
    v: &str,
) -> Result<Option<Box<dyn TextMapPropagator + Send + Sync>>, TraceError> {
    match v {
        "tracecontext" => Ok(Some(Box::new(TraceContextPropagator::new()))),
        "baggage" => Ok(Some(Box::new(BaggagePropagator::new()))),
        #[cfg(feature = "jaeger")]
        "jaeger" => Ok(Some(Box::new(opentelemetry_jaeger::Propagator::default()))),
        "none" => Ok(None),
        unknown => Err(TraceError::from(format!(
            "unsupported propagators form env OTEL_PROPAGATORS: '{unknown}'"
        ))),
    }
}

pub fn init_propagator() -> Result<(), TraceError> {
    let value_from_env =
        std::env::var("OTEL_PROPAGATORS").unwrap_or_else(|_| "tracecontext,baggage".to_string());
    let propagators: Vec<(Box<dyn TextMapPropagator + Send + Sync>, String)> = value_from_env
        .split(',')
        .map(|s| {
            let name = s.trim().to_lowercase();
            propagator_from_string(&name).map(|o| o.map(|b| (b, name)))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    if !propagators.is_empty() {
        let (propagators_impl, propagators_name): (Vec<_>, Vec<_>) =
            propagators.into_iter().unzip();
        tracing::debug!(target: "otel::setup", OTEL_PROPAGATORS = propagators_name.join(","));
        let composite_propagator = TextMapCompositePropagator::new(propagators_impl);
        opentelemetry::global::set_text_map_propagator(composite_propagator);
    }
    Ok(())
}

pub fn build_otel_layer<S>() -> Result<OpenTelemetryLayer<S, Tracer>, TraceError>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or("linkshelf-dev".into());
    let resource = Resource::new(vec![semconv::resource::SERVICE_NAME.string(service_name)]);

    init_propagator()?;

    let mut map = HashMap::new();
    map.insert(
        "authorization".to_string(),
        std::env::var("HYPERDX_API_KEY").unwrap(),
    );

    let otel_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(
                    std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
                        .expect("Missing OTEL_EXPORTER_OTLP_TRACES_ENDPOINT env var"),
                )
                .with_headers(map),
        )
        .with_trace_config(
            sdk::trace::config()
                .with_resource(resource)
                .with_sampler(sdk::trace::Sampler::AlwaysOn),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;

    Ok(tracing_opentelemetry::layer()
        .with_exception_field_propagation(true)
        .with_tracer(otel_tracer))
}

pub fn init_subscribers() -> Result<(), TraceError> {
    //setup a temporary subscriber to log output during setup
    let subscriber = tracing_subscriber::registry()
        .with(build_loglevel_filter_layer())
        .with(build_logger_text());
    let _guard = tracing::subscriber::set_default(subscriber);
    info!("init logging & tracing");

    let subscriber = tracing_subscriber::registry()
        .with(build_otel_layer()?)
        .with(build_loglevel_filter_layer())
        .with(build_logger_text());
    tracing::subscriber::set_global_default(subscriber)
        .expect("Unable to set global tracing subscriber");
    Ok(())
}
