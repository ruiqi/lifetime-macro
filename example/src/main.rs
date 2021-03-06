#![allow(dead_code)]
#![feature(member_constraints)]

use lifetime_derive::lifetime;

fn main() {}

#[lifetime()]
struct Context(&str);

#[lifetime()]
struct Parser {
    context: &Context,
}

#[lifetime()]
impl Parser {
    #[lifetime("self.context.0(0) -> (0)")] // "self.context[Context,0].0(0) => Output!(0)"
    fn parse(&self) -> Result<(), &str> {
        Err(&self.context.0[1..])
    }
}

#[lifetime("context.0(0) -> (0)")] // "context[Context,0].0(0) -> Output!(0)"
fn parse_context(context: Context) -> Result<(), &str> {
    Parser { context: &context }.parse()
}

#[lifetime("x(0), y(0) -> (0)")] // "x(0), y(0) -> Output!(0)"
fn demo0<T, U>(x: &T, y: &T) -> &T {
    if true {
        x
    } else {
        y
    }
}

#[lifetime("x(0), y(0) -> (0, 1)")] // "x(0), y(0) -> Output!(0, 1)"
fn demo1<T, U>(x: &T, y: &T) -> (&T, &T) {
    if true {
        (x, y)
    } else {
        (y, x)
    }
}

#[lifetime("x(0), y(0) -> (0, 1)", "z1, z2 -> (2)")] // "x(0), y(0) -> Output!(0, 1)", "z1(0), z2(0) -> Output!(2)"
fn demo2<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
    if z1 >= z2 {
        (x, y, z1)
    } else {
        (y, x, z2)
    }
}

#[lifetime()]
struct Demo3<G, R> {
    x: &G,
    y: &G,
    z: &R,
}

#[lifetime()]
impl<G, R> Demo3<G, R> {
    #[lifetime("x, y -> (0)")] // "x(0), y(0) -> Output!(0)"
    fn demo3_0<T, U>(&self, x: &T, y: &T) -> &T {
        if true {
            x
        } else {
            y
        }
    }

    #[lifetime("x, y -> (0, 1)")] // "x(0), y(0) -> Output!(0, 1)"
    fn demo3_1<T, U>(&self, x: &T, y: &T) -> (&T, &T) {
        if true {
            (x, y)
        } else {
            (y, x)
        }
    }

    #[lifetime("x, y -> (0, 1)", "z1, z2 -> (1, 2)")] // "x(0), y(0) -> Output!(0, 1)", "z1(0), z2(0) -> Output!(1, 2)"
    fn demo3_2<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
        if z1 >= z2 {
            (x, y, z1)
        } else {
            (y, x, z2)
        }
    }
}

#[lifetime()]
struct Demo4<G, R> {
    x: &G,
    y: &G,
    z: &R,
}

#[lifetime()]
impl<G, R> Demo4<G, R> {
    #[lifetime("x -> self.x", "y -> self.y", "z -> self.z")] // "x(0) -> self.x(0)", "y(0) -> self.y(0)", "z(0) -> self.z(0)"
    fn demo4_0(x: &G, y: &G, z: &R) -> Self {
        Self { x: x, y: y, z: z }
    }

    #[lifetime("x -> self.x", "y -> self.y", "z -> self.z")] // "x(0) -> self.x(0)", "y(0) -> self.y(0)", "z(0) -> self.z(0)"
    fn demo4_1(x: &G, y: &G, z: &R) -> i64 {
        Self { x: x, y: y, z: z };

        18
    }

    #[lifetime("x -> self.x", "z -> self.z", "self -> (0)")] // "x(0) -> self.x(0)", "z(0) -> self.z(0)", "self(0) -> Output!(0)"
    fn demo4_2(&mut self, x: &G, z: &R) -> &Self {
        self.x = x;
        self.z = z;
        self
    }

    #[lifetime(
        "x -> self.x -> (1)",  // "x(0) -> self.x(0) -> Output!(1)"
        "y -> self.y -> (1)",  // "y(0) -> self.y(0) -> Output!(1)"
        "z -> self.z, (2)",  // "z(0) -> self.z(0), Output!(2)"
        "self -> (0)"  // "self(0) -> Output!(0)"
    )]
    fn demo4_3(&mut self, x: &G, y: &G, z: &R) -> (&Self, &G, &R) {
        self.x = x;
        self.y = y;
        self.z = z;

        if true {
            (self, self.x, z)
        } else {
            (self, self.y, z)
        }
    }
}

#[lifetime()]
struct Deom5<G> {
    x: &G,
    y: &G,
    z: &G,
}

#[lifetime()]
impl<G> Deom5<G> {
    #[lifetime(
        "x, self.x -> (0, 1)"  // "x(0), self.x(0) -> Output!(0, 1)"
        "y, self.y -> (0, 1)"  // "y(0), self.y(0) -> Output!(0, 1)"
        "z, self.z -> (0, 1)"  // "z(0), self.z(0) -> Output!(0, 1)"
    )]
    fn demo5_0(&mut self, x: &G, y: &G, z: &G) -> (&G, &G) {
        match 0 {
            0 => (self.x, x),
            1 => (self.y, y),
            2 => (self.z, z),
            3 => (x, self.x),
            4 => (y, self.y),
            5 => (z, self.z),
            _ => unreachable!(),
        }
    }

    #[lifetime(
        "x -> self.x -> (0, 1)"  // "x(0) -> self.x(0) -> Output!(0, 1)"
        "y -> self.y -> (0, 1)"  // "x(0) -> self.y(0) -> Output!(0, 1)"
        "z -> self.z -> (0, 1)"  // "z(0) -> self.z(0) -> Output!(0, 1)"
    )]
    fn demo5_1(&mut self, x: &G, y: &G, z: &G) -> (&G, &G) {
        self.x = x;
        self.y = y;
        self.z = z;

        match 0 {
            0 => (self.x, x),
            1 => (self.y, y),
            2 => (self.z, z),
            3 => (x, self.x),
            4 => (y, self.y),
            5 => (z, self.z),
            _ => unreachable!(),
        }
    }

    #[lifetime("x -> self.x -> (0)", "y -> self.y -> (0)", "z -> self.z -> (0)")] // "x(0) -> self.x(0) -> Output!(0)", "y(0) -> self.y(0) -> Output!(0)", "z(0) -> self.z(0) -> Output!(0)"
    fn demo5_2(&self, x: &G, y: &G, z: &G) -> &G {
        let demo5 = Self { x: x, y: y, z: z };

        match 0 {
            0 => x,
            1 => y,
            2 => z,
            3 => demo5.x,
            4 => demo5.y,
            5 => demo5.z,
            _ => unreachable!(),
        }
    }
}

#[lifetime()]
struct Demo6A<T, U>(&T, &U);

#[lifetime()]
enum Demo6B<T, U> {
    Single(&Demo6A<T, U>),
    Double((&Demo6A<T, U>, &Demo6A<T, U>)),
    Multiple(&Vec<&Demo6A<T, U>>),
}

#[lifetime()]
struct Demo6C<T, U> {
    b: Demo6B<T, U>,
}

#[lifetime()]
impl<T, U> Demo6C<T, U> {
    #[lifetime(
        "self.b.Single(0), self.b.Double(0), self.b.Multiple(1) -> (0)"  // "self.b[Demo6B,0].Single(0), self.b[Demo6B,0].Double(0), self.b[Demo6B,0].Multiple(1) -> Output!(0)"
        "self.b.Single.0(0), self.b.Double[Demo6A,0].0(0), self.b.Multiple.0(0) -> Output!.0(0)", // "self.b[Demo6B,0].Single[Demo6A,0].0(0), self.b[Demo6B,0].Double[Demo6A,0].0(0), self.b[Demo6B,0].Multiple[Demo6A,0].0(0) -> Output![Demo6A,0].0(0)"
        "self.b.Single.1(0), self.b.Double[Demo6A,0].1(0), self.b.Multiple.1(0) -> Output!.1(0)", // "self.b[Demo6B,0].Single[Demo6A,0].1(0), self.b[Demo6B,0].Double[Demo6A,0].1(0), self.b[Demo6B,0].Multiple[Demo6A,0].1(0) -> Output![Demo6A,0].1(0)"
    )]
    fn first(&self) -> Option<&Demo6A<T, U>> {
        match self.b {
            Demo6B::Single(single) => Some(single),
            Demo6B::Double(double) => Some(double.0),
            Demo6B::Multiple(multiple) => multiple.first().map(|a| *a),
        }
    }
}

#[lifetime(
    "x -> y -> (0)"
)]
fn f(x: &i32, mut y: &i32) -> &i32 {
    y = x;

    y
}

#[lifetime()]
trait Demo7T<T> {
    fn new(x: &T) -> Self
    where
        Self: Sized;

    fn demo7t_0(&self, t1: &T, t2: &T) -> &T;
}

#[lifetime()]
struct Demo7A<T> {
    x: &T,
}

#[lifetime()]
impl<T> Demo7T<T> for Demo7A<T>
where
    T: Ord,
{
    #[lifetime(
        "x -> self.x"
    )]
    fn new(x: &T) -> Self {
        Demo7A { x: x }
    }

    #[lifetime(
        "t1, t2, self.x -> (0)"
    )]
    fn demo7t_0(&self, t1: &T, t2: &T) -> &T {
        &[self.x, t1, t2].iter().max().unwrap()
    }
}

#[lifetime()]
struct Deom7B<T> {
    x: &T,
}

#[lifetime()]
impl<T> Demo7T<T> for Deom7B<T>
where
    T: Ord,
{
    #[lifetime(
        "x -> self.x"
    )]
    fn new(x: &T) -> Self {
        Deom7B { x: x }
    }

    #[lifetime(
        "t1, t2, self.x -> (0)"
    )]
    fn demo7t_0(&self, t1: &T, t2: &T) -> &T {
        &[t1, t2].iter().min().unwrap()
    }
}

#[lifetime(
    // "x -> Output![Demo7T,0].new/x, Output![Demo7T,1].new/x"
)]
fn demo7c<T, U1: Demo7T<T>, U2: Demo7T<T>>(x: &T, y: &T) -> (U1, U2)
where
    T: Ord,
{
    (U1::new(x), U2::new(x))
}

/*
fn demo7cX<'f_a1, 'f_b1, 'f_c1, 'f_d1, 'f_e1, 'f_a2, 'f_b2, 'f_c2, 'f_d2, 'f_e2,'f_f: 'f_a2, T, U1: Demo7T<'f_a1, 'f_b1, 'f_c1, 'f_d1, 'f_e1, T>, U2: Demo7T<'f_a2, 'f_b2, 'f_c2, 'f_d2, 'f_e2, T>>(
    x: &'f_f T,
) -> U2
where
    T: Ord,
{
    Demo7T::new(x)
}

#[lifetime()]
fn demo7d<'a, T>(x: &'a T) -> impl Demo7T<'a, T>
where
    T: Ord,
{
    Demo7A { x: x }
}

fn demo7e<'a, T>(x: &'a T) -> Box<dyn 'a + Demo7T<'a, T>>
where
    T: Ord,
{
    if true {
        Box::new(Demo7A { x: x })
    } else {
        Box::new(Deom7B { x: x })
    }
}
*/

/*
#[lifetime("(0, 1) -> (2)")]
fn demo8<T>() -> fn(&T, &T) -> &T {
    |a, b| {
        if true {
            a
        } else {
            b
        }
    }
}


#[lifetime("(1, 2) -> (3)")]
fn demo9<T>() -> &impl Fn(&T, &T) -> &T {
    &|a, b| {
        if true {
            a
        } else {
            b
        }
    }
}
*/

/*
struct Deom7_S {}

impl<'a> Demo7_T for &'a Deom7_S {}

trait Foo {}

impl<'a> Foo for &'a() {}

impl<'a> Foo for Box<dyn 'a + Foo> {}

/*
#[lifeitime(
    "x(1) -> "
)]
*/
fn through_box<'a>(x: &'a ()) -> impl 'a + Foo {
    let box_: Box<dyn Foo> = Box::new(x);
    box_
}

/*
trait Foo {}

impl Foo for () {}

impl Foo for Box<dyn Foo> {}

fn through_box(x: ()) -> impl Foo {
    let box_: Box<dyn Foo> = Box::new(x);
    box_
}
*/
*/

fn fix_cargo_expand_bug() {}
