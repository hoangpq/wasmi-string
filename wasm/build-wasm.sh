#!/usr/bin/env bash
 rustc +nightly --crate-type cdylib --target wasm32-unknown-unknown test.rs
