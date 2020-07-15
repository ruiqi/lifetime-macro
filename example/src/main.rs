use lifetime_derive::lifetime;

fn main() {
    let x = vec![1, 3, 5];
    let y = vec![2, 4, 6];
    let z1 = 1;
    let z2 = 2;

    println!("demo1: {:?}", demo1(&x, &y, &z1, &z2));
    println!("demo2: {:?}", demo2(&x, &y, &z1, &z2));
    println!("demo3: {:?}", demo3(&x, &y, &z1, &z2));
    let demo4 = Demo4 {
        x: &1,
        y: &3,
        z: &&vec![1, 2, 3],
    };
    println!("demo4_1: {:?}", demo4.demo4_1(&x, &y, &z1, &z2));
    println!("demo4_2: {:?}", demo4.demo4_2(&x, &y, &z1, &z2));
    println!("demo4_3: {:?}", demo4.demo4_3(&x, &y, &z1, &z2));
}

#[lifetime("0, 1: x, y")]
fn demo1<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
    if z1 >= z2 {
        (x, y)
    } else {
        (y, x)
    }
}

#[lifetime("0: x, y")]
fn demo2<T, U: PartialOrd>(x: &T, y: &T, z1: &U, z2: &U) -> &T {
    if z1 >= z2 {
        x
    } else {
        y
    }
}

#[lifetime("0, 1: x, y", "2: z1, z2")]
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

impl<G, R> Demo4<G, R> {
    #[lifetime("0, 1: x, y")]
    fn demo4_1<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T) {
        if z1 >= z2 {
            (x, y)
        } else {
            (y, x)
        }
    }

    #[lifetime("0: x, y")]
    fn demo4_2<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> &T {
        if z1 >= z2 {
            x
        } else {
            y
        }
    }

    #[lifetime("0, 1: x, y", "2: z1, z2")]
    fn demo4_3<T, U: PartialOrd>(&self, x: &T, y: &T, z1: &U, z2: &U) -> (&T, &T, &U) {
        if z1 >= z2 {
            (x, y, z1)
        } else {
            (y, x, z2)
        }
    }
}