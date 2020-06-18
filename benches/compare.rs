// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use criterion::{criterion_group, criterion_main, Criterion};

fn rustracing_harness() {
    fn dummy_rustracing(span: &rustracing::span::Span<()>) {
        for _ in 0..99 {
            let _child_span = span.child("child", |c| c.start_with_state(()));
        }
    }

    let (span_tx, span_rx) = crossbeam::channel::bounded(100);

    {
        let tracer = rustracing::Tracer::with_sender(rustracing::sampler::AllSampler, span_tx);
        let parent_span = tracer.span("parent").start_with_state(());
        dummy_rustracing(&parent_span);
    }

    let _r = span_rx.iter().collect::<Vec<_>>();
}

fn init_opentelemetry() {
    use opentelemetry::api::Provider;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::Registry;

    let tracer = opentelemetry::sdk::Provider::default().get_tracer("component_name");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default().with(telemetry).init();
}

fn opentelemetry_harness() {
    fn dummy_opentelementry() {
        for _ in 0..99 {
            let child = tracing::span!(tracing::Level::TRACE, "child");
            let _enter = child.enter();
        }
    }

    let root = tracing::span!(tracing::Level::TRACE, "parent");
    let _enter = root.enter();

    dummy_opentelementry();
}

fn minitrace_harness() {
    const PARENT: u32 = 0;
    const CHILD: u32 = 1;

    fn dummy_minitrace() {
        for _ in 0..99 {
            let _guard = minitrace::new_span(CHILD);
        }
    }

    let (root, collector) = minitrace::trace_enable(PARENT);

    {
        let _guard = root;
        dummy_minitrace();
    }

    let _r = collector.collect();
}

#[derive(Debug)]
enum TracingType {
    TokioTracing,
    Rustracing,
    Minitrace,
}

fn tracing_comparison(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "tracing_comparison",
        |b, tp| {
            b.iter(|| match tp {
                TracingType::TokioTracing => opentelemetry_harness(),
                TracingType::Rustracing => rustracing_harness(),
                TracingType::Minitrace => minitrace_harness(),
            });
        },
        &[
            TracingType::TokioTracing,
            TracingType::Rustracing,
            TracingType::Minitrace,
        ],
    );
}

fn rustracing_multi(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "rustracing_multi",
        |b, thread_cnt| {
            b.iter(|| {
                let cnt = 1000;
                let (span_tx, span_rx) = crossbeam::channel::bounded(*thread_cnt * cnt);
                let tracer = rustracing::Tracer::with_sender(rustracing::sampler::AllSampler, span_tx);
                let parent_span = std::sync::Arc::new(tracer.span("parent").start_with_state(()));
            
                for _ in 0..*thread_cnt {
                    let parent_span = parent_span.clone();
                    std::thread::spawn(move || {
                        for _ in 0..cnt {
                            let _child_span = parent_span.child("child", |c| c.start_with_state(()));
                        }
                    });
                }

                drop(parent_span);
                drop(tracer);
                let _r = span_rx.iter().collect::<Vec<_>>();
            });
        },
        vec![4, 8, 16, 32],
    );
}

fn tokio_tracing_multi(c: &mut Criterion) {
    init_opentelemetry();

    #[tracing::instrument]
    async fn f() {
        for _ in 0..1000 {
            let child = tracing::info_span!("child");
            let _enter = child.enter();
        }
    }

    c.bench_function_over_inputs(
        "tokio_tracing_multi",
        |b, thread_cnt| {
            b.iter(|| {
                let wg = crossbeam::sync::WaitGroup::new();

                let root = tracing::info_span!("parent");
                let _enter = root.enter();

                for _ in 0..*thread_cnt {
                    let wg = wg.clone();

                    std::thread::spawn(move || {
                        futures_03::executor::block_on(f());
                        drop(wg);
                    });
                }

                wg.wait();
            });
        },
        vec![4, 8, 16, 32],
    );
}

criterion_group!(benches, tracing_comparison, rustracing_multi, tokio_tracing_multi);
criterion_main!(benches);
