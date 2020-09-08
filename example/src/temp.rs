#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
use lifetime_derive::{lifetime, lifetime_nothing};

fn main() {}

struct GGG<'s_a, 's_b, G> {
    g: &'s_a &'s_b G,
}

struct Context<'s_a, T, G> {
    t: &'s_a T,
    e: Result<GGG<G>, ()>,
}

struct Parser<'s_a, 's_b, T, G> {
    t: &'s_a T,
    context: &Context<T, G>,
}