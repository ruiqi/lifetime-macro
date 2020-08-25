use lifetime_derive::lifetime;

fn main() {}

/*
#[lifetime("0, 1 -> x, y")]
fn demo1<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
    if z1 >= z2 {
        (x, y)
    } else {
        (y, x)
    }
}

#[lifetime("0 -> x, y")]
fn demo2<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> &T {
    if z1 >= z2 {
        x
    } else {
        y
    }
}

#[lifetime("0, 1 -> x, y", "2 -> z1, z2")]
fn demo3<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
    if z1 >= z2 {
        (x, y, z1)
    } else {
        (y, x, z2)
    }
}

#[lifetime("x, y, z")]
struct Demo4<G, R> {
    x: &G,
    y: &G,
    z: &R,
}

#[lifetime()]
impl<G, R> Demo4<G, R> {
    #[lifetime("0, 1 -> x, y")]
    fn demo4_1<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
        if z1 >= z2 {
            (x, y)
        } else {
            (y, x)
        }
    }

    #[lifetime("0 -> x, y")]
    fn demo4_2<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> &T {
        if z1 >= z2 {
            x
        } else {
            y
        }
    }

    #[lifetime("0, 1 -> x, y", "2 -> z1, z2")]
    fn demo4_3<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
        if z1 >= z2 {
            (x, y, z1)
        } else {
            (y, x, z2)
        }
    }
}

#[lifetime("x, y", "z")]
struct Demo5<G, R> {
    x: &G,
    y: &G,
    z: &R,
}

//#[lifetime()]
impl<G, R> Demo5<G, R> {
    /*
    // .x -> x, .y -> y, .z -> z
    fn demo5_1(x: &'a G, y: &'a G, z: &'b R) -> Self {
        Self { x: x, y: y, z: z }
    }

    // .x -> x, .z -> z
    fn demo5_2(&mut self, x: &'a G, z: &'b R) -> &Self {
        self.x = x;
        self.z = z;
        self
    }

    // (0) -> .x, .y -> x, y
    fn demo5_3(&mut self, x: &'a G, y: &'a G, z: &'b R) -> &'a G {
        self.x = x;
        self.y = y;
        self.z = z;

        if 3 > 1 {
            self.x
        } else {
            self.y
        }
    }

    // (1) -> .x -> x, .z -> z, (2) -> o, .y - y,
    fn demo5_5<'c>(&mut self, x: &'a G, y: &'a G, z: &'b R, o: &'c G) -> (&Self, &'a G, &'c G) {
        self.x = x;
        self.y = y;
        self.z = z;

        (self, self.x, o)
    }
    */
}

struct Deom6<'a, 'b, 'c, G> {
    x: &'a G,
    y: &'b G,
    z: &'c G,
}

impl<'a, 'b, 'c, G> Deom6<'a, 'b, 'c, G> {
    // (0, 1) -> .x, .y, .z -> x, y, z
    fn demo6_1<'d, 'f>(&mut self, x: &'a G, y: &'b G, z: &'c G, p: &'d G, o: i64) -> (&G, &G) {
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

    // (0) -> .x, .y, .z -> x, y, z
    fn demo6_2(&self, x: &'a G, y: &'b G, z: &'c G, o: i64) -> &G {
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

struct Context<'s>(&'s str);

// #lifetime(".context(0)", ".context(1), context2(0)")
struct Parser<'c, 's> {
    context: &'c Context<'s>,
}

// #lifetime()
impl<'c, 's: 'q + 'e, 'q, 'e> Parser<'c, 's> {
    // #lifetime("(0) -> .context(1)")
    fn parse(&self) -> Result<(), &'q str> {
        Err(&self.context.0[1..])
    }

    // #lifetime("(0) -> .context(1), .context(0) -> context(0), context2(1)")
    fn parse_context<'g>(context: Context<'g>) -> Result<(), &'g str> {
        Parser { context: &context }.parse()
    }
}

fn test1<'a: 'd, 'b: 'd, 'c: 'd, 'd, T>(x: &'a T, y: &'b T, z: &'c T) -> &'d T {
    let n = 3;
    match n {
        1 => x,
        2 => y,
        3 => z,
        _ => unreachable!(),
    }
}
*/

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

/*
struct Test3<'b> {
    x: Option<&'b Self>,
}

impl<'b, 'd2: 'b> Test3<'b> {
    //fn test3_1<'c1: 'b>(&'c1 mut self) {
    //    self.x = Some(self);
    //}

    fn test3_2(&mut self, t: &'d2 Test3) {
        self.x = Some(t);
    }
}
*/

struct Test4 {}
