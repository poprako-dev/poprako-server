use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use poprako_server::{UserComplex, benchmark};

fn benchmark_password_operations(criterion: &mut Criterion) {
    let password = "benchmark-password";
    let runtime = tokio::runtime::Runtime::new()
        .expect("benchmark runtime must initialize");
    let password_hash = runtime
        .block_on(UserComplex::hash_password(password))
        .expect("benchmark fixture password must hash");

    let mut hash_group = criterion.benchmark_group("password_hash");
    hash_group.bench_function("argon2id", |bencher| {
        bencher.iter(|| {
            runtime
                .block_on(UserComplex::hash_password(black_box(password)))
                .expect("password hashing must succeed");
        });
    });
    hash_group.finish();

    let mut verify_group = criterion.benchmark_group("password_verify");
    verify_group.bench_function("argon2id", |bencher| {
        bencher.iter(|| {
            assert!(runtime.block_on(UserComplex::verify_password(
                black_box(password),
                black_box(&password_hash),
            )));
        });
    });
    verify_group.finish();
}

fn benchmark_aggregation_operations(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new()
        .expect("benchmark runtime must initialize");

    let mut archive_group = criterion.benchmark_group("archive");
    archive_group.bench_function("prepare_write_8x8x48", |bencher| {
        bencher.iter_batched(
            || benchmark::archive_input().expect("archive input must build"),
            |archive_input| {
                assert!(
                    runtime.block_on(benchmark::prepare_archive(archive_input))
                );
            },
            BatchSize::SmallInput,
        );
    });
    archive_group.finish();

    let mut import_group = criterion.benchmark_group("chapter_import");
    import_group.bench_function("label_plus_64x_material", |bencher| {
        bencher.iter(|| assert!(benchmark::parse_label_plus()));
    });
    import_group.bench_function("poprako_2000_units", |bencher| {
        bencher.iter(|| assert!(benchmark::parse_poprako()));
    });
    import_group.finish();

    let mut export_group = criterion.benchmark_group("chapter_export");
    let label_plus_export_input = benchmark::label_plus_export_input();
    export_group.bench_function("label_plus_64x48", |bencher| {
        bencher.iter(|| {
            assert!(benchmark::make_label_plus(&label_plus_export_input));
        });
    });
    export_group.finish();

    let mut unit_group = criterion.benchmark_group("unit");
    unit_group.bench_function("build_index_updates_10000", |bencher| {
        bencher.iter_batched(
            benchmark::unit_index_input,
            |unit_index_input| {
                assert!(benchmark::build_unit_index_updates(unit_index_input));
            },
            BatchSize::SmallInput,
        );
    });
    unit_group.finish();
}

criterion_group!(
    benches,
    benchmark_password_operations,
    benchmark_aggregation_operations,
);
criterion_main!(benches);
