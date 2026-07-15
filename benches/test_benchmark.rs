use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use poprako_server::UserComplex;

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

criterion_group!(benches, benchmark_password_operations);
criterion_main!(benches);
