// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn dummy_iter(i: usize) {
    #[minitrace::trace(0u32)]
    fn dummy() {}

    for _ in 0..i - 1 {
        dummy();
    }
}

#[minitrace::trace(0u32)]
fn dummy_rec(i: usize) {
    if i > 1 {
        dummy_rec(i - 1);
    }
}

fn trace_wide_bench(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "trace_wide",
        |b, len| {
            b.iter(|| {
                let (root, collector) = minitrace::trace_enable(0u32);
                {
                    let _guard = root;

                    if *len > 1 {
                        dummy_iter(*len);
                    }
                }

                let _r = black_box(collector.collect());
            });
        },
        vec![1, 10, 100, 1000, 10000],
    );
}

fn trace_deep_bench(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "trace_deep",
        |b, len| {
            b.iter(|| {
                let (root, collector) = minitrace::trace_enable(0u32);

                {
                    let _guard = root;

                    if *len > 1 {
                        dummy_rec(*len);
                    }
                }

                let _r = black_box(collector.collect());
            });
        },
        vec![1, 10, 100, 1000, 10000],
    );
}

fn trace_multi_thread(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "per_thread_collect",
        |b, thread_cnt| {
            b.iter(|| {
                let (root, collector) = minitrace::trace_enable(0u32);

                let mut threads = Vec::with_capacity(*thread_cnt);

                for i in 0..*thread_cnt {
                    let handle = minitrace::trace_crossthread(i as u32);
                    let join_handle = std::thread::spawn(move || {
                        let mut handle = handle;
                        let _g = handle.trace_enable();

                        for i in 0..1000 {
                            let _g = minitrace::new_span(i as u32);
                        }
                    });
                    threads.push(join_handle);
                }

                for handle in threads {
                    let _ = handle.join();
                }

                drop(root);
                let _span_sets = collector.collect();
            });
        },
        vec![4, 8, 16, 32],
    );
}

criterion_group!(benches, trace_wide_bench, trace_deep_bench, trace_multi_thread);
criterion_main!(benches);
