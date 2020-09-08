use lifetime_derive::{lifetime, lifetime_nothing};

fn main() {}

/*
#[lifetime("x, y -> (0), (1)")]
fn demo1<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
    if z1 >= z2 {
        (x, y)
    } else {
        (y, x)
    }
}

#[lifetime("x, y -> (0)")]
fn demo2<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> &T {
    if z1 >= z2 {
        x
    } else {
        y
    }
}

#[lifetime("x, y -> (0), (1)", "z1, z2 -> (2)")]
fn demo3<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
    if z1 >= z2 {
        (x, y, z1)
    } else {
        (y, x, z2)
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
    #[lifetime("x, y -> (0), (1)")]
    fn demo4_1<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
        if z1 >= z2 {
            (x, y)
        } else {
            (y, x)
        }
    }

    #[lifetime("x, y -> (0)")]
    fn demo4_2<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> &T {
        if z1 >= z2 {
            x
        } else {
            y
        }
    }

    #[lifetime("x, y -> (0), (1)", "z1, z2 -> (1), (2)")]
    fn demo4_3<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
        if z1 >= z2 {
            (x, y, z1)
        } else {
            (y, x, z2)
        }
    }
}

#[lifetime()]
struct Demo5<G, R> {
    x: &G,
    y: &G,
    z: &R,
}

#[lifetime()]
impl<G, R> Demo5<G, R> {
    #[lifetime(
        "x -> self.x",
        "y -> self.y",
        "z -> self.z",
    )]
    fn demo5_0(x: &G, y: &G, z: &R) -> Self  {
        Self { x: x, y: y, z: z }
    }

    #[lifetime(
        "x -> self.x",
        "y -> self.y",
        "z -> self.z",
    )]
    fn demo5_1(x: &G, y: &G, z: &R) -> i64  {
        Self { x: x, y: y, z: z };

        18
    }

    #[lifetime(
        "self -> (0)",
        "x -> self.x",
        "z -> self.z",
    )]
    fn demo5_2(&mut self, x: &G, z: &R) -> &Self {
        self.x = x;
        self.z = z;
        self
    }

    #[lifetime(
        "x -> self.x -> (0)",
        "y -> self.y -> (0)",
        "z -> self.z",
    )]
    fn demo5_3(&mut self, x: &G, y: &G, z: &R) -> &G {
        self.x = x;
        self.y = y;
        self.z = z;

        if 3 > 1 {
            self.x
        } else {
            self.y
        }
    }

    #[lifetime(
        "self -> (0)",
        "x -> self.x -> (1)",
        "y -> self.y",
        "z -> self.z",
        "o -> (2)",
    )]
    fn demo5_5(&mut self, x: &G, y: &G, z: &R, o: &G) -> (&Self, &G, &G) {
        self.x = x;
        self.y = y;
        self.z = z;

        (self, self.x, o)
    }
}

#[lifetime()]
struct Deom6<G> {
    x: &G,
    y: &G,
    z: &G,
}

#[lifetime()]
impl<G> Deom6<G> {
    #[lifetime(
        "x -> self.x -> (0), (1)"
        "y -> self.y -> (0), (1)"
        "z -> self.z -> (0), (1)"
    )]
    fn demo6_1(&mut self, x: &G, y: &G, z: &G, p: &G, o: i64) -> (&G, &G) {
        self.x = x;
        self.y = y;
        self.z = z;

        match o {
            1 => (self.x, x),
            2 => (self.y, y),
            3 => (x, self.x),
            4 => (y, self.y),
            5 => (z, self.z),
            6 => (self.z, z),
            //7 => (p, x),
            _ => unreachable!(),
        }
    }

    #[lifetime(
        "x -> self.x -> (0)",
        "y -> self.y -> (0)",
        "z -> self.z -> (0)",
    )]
    fn demo6_2(&self, x: &G, y: &G, z: &G, o: i64) -> &G {
        let temp = Self { x: x, y: y, z: z };

        match o {
            1 => x,
            2 => y,
            3 => temp.x,
            4 => temp.y,
            5 => temp.z,
            6 => z,
            7 => x,
            _ => unreachable!(),
        }
    }
}
*/

#[lifetime()]
struct GGG<G> {
    g: &&G,
}

/*
#[lifetime()]
struct Context<T, G> {
    t: &T,
    e: Result<GGG<G>, ()>,
}

#[lifetime()]
struct Parser<T, G> {
    t: &T,
    context: &Context<T, G>,
}
*/

struct XX<'a, 'b, 'c> {
    x: GGG<'a, 'b, &'c str>,
}

/*
#[lifetime()]
impl<T, G>
    Parser<T, G>
{
    #[lifetime(
        "self.context(1) -> (0)",
        "self.context.e.g(0) -> (1)",
        "self.context.e.g(1) -> (2)",
    )]
    fn parse(&self) -> Result<(), (&T, &&G)> {
        if self.context.e.is_ok() {
            Err((self.context.t, self.context.e.as_ref().ok().unwrap().g))
        } else {
            unreachable!()
        }
    }

    #[lifetime(
        "context1.t(0) -> (0)",
        "context1.u.g(0) -> (1)",
        "context1.u.g(1) -> (2)",
    )]
    fn parse_context(t: T, context1: Context<T, G>) -> Result<(), (&T, &&G)> {
        Parser {
            t: &t,
            context: &context1
        }.parse()
    }
}
*/

/*
//#[lifetime()]
struct Context<'a, 'b, 'c, T, G> {
    t: &'a T,
    e: Result<GGG<'b, 'c, G>, ()>,
}

//#[lifetime()]
struct Parser<'a, 'b, 'c, 'd, 'e, T, G> {
    t: &'a T,
    context: &'b Context<'c, 'd, 'e, T, G>,
}

//#[lifetime()]
impl<'a, 'b, 'c: 'g, 'd: 'h, 'e: 'i, 'f, 'g, 'h, 'i, 'j: 'p, 'k: 'q, 'l: 'r, 'p, 'q, 'r, T, G>
    Parser<'a, 'b, 'c, 'd, 'e, T, G>
{
    //#[lifetime(
    //    "self.context.t(0) -> (0)",
    //    "self.context.e.g(0) -> (1)",
    //    "self.context.e.g(1) -> (2)",
    //)]
    fn parse(&'f self) -> Result<(), (&'g T, &'h &'i G)> {
        if self.context.e.is_ok() {
            Err((self.context.t, self.context.e.as_ref().ok().unwrap().g))
        } else {
            unreachable!()
        }
    }

    //#[lifetime(
    //    "context1.t(0) -> (0)",
    //    "context1.u.g(0) -> (1)",
    //    "context1.u.g(1) -> (2)",
    //)]
    fn parse_context(t: T, context1: Context<'j, 'k, 'l, T, G>) -> Result<(), (&'p T, &'q &'r G)> {
        Parser {
            t: &t,
            context: &context1
        }.parse()
    }
}
*/

/*
struct XContext<'a>(&'a str);

struct XParser<'a, 'b> {
    context: &'a XContext<'b>,
}

impl<'a, 'b: 'd, 'c, 'd, 'e: 'f, 'f> XParser<'a, 'b> {
    // self.context(1) -> (0)
    fn xparse(&'c self) -> Result<(), &'d str> {
        Err(&self.context.0[1..])
    }

    // context.s(0) -> (0)
    fn xparse_context(context: XContext<'e>) -> Result<(), &'f str> {
        XParser { context: &context }.xparse()
    }
}
*/

/*
#[lifetime()]
fn test1(x: &T, y: &T, z: &T) -> &T {
    let n = 3;
    match n {
        1 => x,
        2 => y,
        3 => z,
        _ => unreachable!(),
    }
}
*/

/*
#[lifetime()]
struct Test2<T> {
    x: &T,
    y: &&T,
    z: &T,
}

#[lifetime()]
impl<T> Test2<T> {
    #[lifetime("x, y -> (0, 1)")]
    fn test2_1(x: &T, y: &T) -> (&T, &T) {
        let n = 3;
        match n {
            1 => (x, x),
            2 => (y, y),
            _ => unreachable!(),
        }
    }

    #[lifetime(
        "self -> (0)",
        "x(0), self.x(0) -> (1)",
        "y(0), self.y(0) -> (2)",
        "y(1), self.y(1) -> (3)"
    )]
    fn test2_2(&self, x: &T, y: &&T) -> (&Self, &T, &&T) {
        let n = 3;
        match n {
            1 => (self, x, y),
            2 => (self, self.x, self.y),
            _ => unreachable!(),
        }
    }

    #[lifetime("x(0) -> self.x(0)", "y(0) -> self.y(0)", "y(1) -> self.y(1)")]
    fn test2_3(&mut self, x: &T, y: &&T) {
        self.x = x;
        self.y = y;
    }

    #[lifetime(
        "x(0) -> self.x(0)",
        "y(0) -> self.y(0)",
        "y(1) -> self.y(1)",
        "self -> (0)",
        "x(0), self.x(0) -> (1)",
        "y(0), self.y(0) -> (2)",
        "y(1), self.y(1) -> (3)"
    )]
    fn test2_4(self: &mut Self, x: &T, y: &&T) -> (&Self, &T, &&T) {
        self.x = x;
        self.y = y;

        let n = 3;
        match n {
            1 => (self, x, y),
            2 => (self, self.x, self.y),
            _ => unreachable!(),
        }
    }

    #[lifetime(
        "self.x -> self.z",
        "self.z -> self.x",
    )]
    fn test2_5(&mut self) {
        let temp = self.x;
        self.x = self.z;
        self.z = temp;
    }

    #[lifetime(
        "x(0) -> self.x(0)",
        "y(0) -> self.y(0)",
        "y(1) -> self.y(1)",
        "x, self.x -> (0)"
        "y -> (1)"
        "y(1) -> (2)"
    )]
    fn test2_6(mut self: Self, x: &T, y: &&T) -> (Self, &T, &&T){
        self.x = x;
        self.y = y;

        let n = 3;
        match n {
            1 => (self, x, y),
            2 => (self, x, y),
            _ => unreachable!(),
        }
    }
}
*/

/*
struct Test3 {
    x: Option<&Self>,
}

impl<, 2: > Test3 {
    //fn test3_1<1: >(&1 mut self) {
    //    self.x = Some(self);
    //}

    fn test3_2(&mut self, t: &2 Test3) {
        self.x = Some(t);
    }
}
*/

//struct Test4 {}
