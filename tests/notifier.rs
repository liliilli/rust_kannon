use kannon::notifier::notifier;
use rand::Rng;

#[derive(Copy, Clone, Debug)]
struct Subject1(i32);

struct Test1(i32, i32, i32, i32, i32, i32, i32, i32);
impl Test1 {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        Self(
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
            rng.gen_range(-100..=100),
        )
    }

    fn test_method(
        &self,
        a: Subject1,
        b: i32,
        c: i32,
        d: i32,
        e: i32,
        f: i32,
        g: i32,
        h: Subject1,
    ) {
        println!(
            "From Test1, {}, {}, {}, {}, {}, {}, {}, {}",
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7
        );
        println!(
            "And from input!, {}, {}, {}, {}, {}, {}, {}, {}",
            a.0, b, c, d, e, f, g, h.0
        );
    }
}

#[test]
fn test1() {
    let mut notifier =
        notifier::Notifier8::<Subject1, i32, i32, i32, i32, i32, i32, Subject1>::new();
    let _event1 = notifier.register_closure(|a, b, c, d, e, f, g, h| {
        println!(
            "From event1, {}, {}, {}, {}, {}, {}, {}, {}",
            a.0, b, c, d, e, f, g, h.0
        );
    });

    let test1 = Test1::new();
    let _event2 = notifier.register_method(&test1, Test1::test_method);

    let mut rng = rand::thread_rng();
    let subject1: Subject1 = Subject1(65539);
    notifier.invoke(
        subject1,
        rng.gen_range(-100..=100),
        rng.gen_range(-100..=100),
        rng.gen_range(-100..=100),
        rng.gen_range(-100..=100),
        rng.gen_range(-100..=100),
        rng.gen_range(-100..=100),
        subject1,
    );
}
