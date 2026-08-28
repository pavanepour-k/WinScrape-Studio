//! Benchmarks for the core DSL pipeline: generating a scrape plan,
//! validating it, and round-tripping it through YAML.
//!
//! These are the operations that run on every "describe what to scrape"
//! request before any network I/O happens, so their cost sets a floor on
//! how responsive the app feels while a user is iterating on a plan.
//!
//! Run with: cargo bench --no-default-features --features http-only

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use winscrape_studio::dsl::generator::DSLGenerator;
use winscrape_studio::dsl::parser::DSLParser;
use winscrape_studio::dsl::DSLValidator;

fn bench_generate_basic(c: &mut Criterion) {
    c.bench_function("dsl_generate_basic", |b| {
        b.iter(|| {
            DSLGenerator::generate_basic(
                black_box("example.com"),
                black_box("https://example.com/products"),
            )
            .unwrap()
        })
    });
}

fn bench_generate_ecommerce(c: &mut Criterion) {
    c.bench_function("dsl_generate_ecommerce", |b| {
        b.iter(|| {
            DSLGenerator::generate_ecommerce(
                black_box("shop.example.com"),
                black_box("https://shop.example.com/catalog"),
            )
            .unwrap()
        })
    });
}

fn bench_validate(c: &mut Criterion) {
    let plan =
        DSLGenerator::generate_ecommerce("shop.example.com", "https://shop.example.com/catalog")
            .expect("fixture plan should generate");
    let validator = DSLValidator::new();

    c.bench_function("dsl_validate", |b| {
        b.iter(|| validator.validate(black_box(&plan)).unwrap())
    });
}

fn bench_yaml_round_trip(c: &mut Criterion) {
    let plan =
        DSLGenerator::generate_ecommerce("shop.example.com", "https://shop.example.com/catalog")
            .expect("fixture plan should generate");
    let yaml = DSLParser::to_yaml(&plan).expect("fixture plan should serialize");

    c.bench_function("dsl_to_yaml", |b| {
        b.iter(|| DSLParser::to_yaml(black_box(&plan)).unwrap())
    });

    c.bench_function("dsl_parse_yaml", |b| {
        b.iter(|| DSLParser::parse_yaml(black_box(&yaml)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_generate_basic,
    bench_generate_ecommerce,
    bench_validate,
    bench_yaml_round_trip
);
criterion_main!(benches);
