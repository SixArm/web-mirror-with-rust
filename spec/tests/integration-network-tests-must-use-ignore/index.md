# Integration network tests must use ignore

Add `#[ignore]` to any integration network test, meaning any test that makes
real network requests, such as running `web-mirror` against a live site.

Give the reason in the attribute, for example:

```rust
#[test]
#[ignore = "needs network access; run with `cargo test -- --ignored`"]
fn test_www_nhs_uk_conditions() {
    // ...
}
```

Why: a plain `cargo test` should be fast, work offline, and not send
requests to other people's servers. Live sites also change, which can make
these tests fail for reasons that have nothing to do with the code.

Run the network tests on purpose:

- Only the network tests: `cargo test -- --ignored`
- All tests: `cargo test -- --include-ignored`
