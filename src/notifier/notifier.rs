use super::event::*;
use paste::paste;

/// Macro for helping declaring `Notifier` type which have various generic types and some methods.
macro_rules! decl_notifier {
    {$cnt:expr, $t:ident $($ts:ident)*} => {
        paste! {
            pub struct [<Notifier $cnt>]<$t, $($ts),*> {
                readys: Vec<[<EventHandle $cnt>]<$t, $($ts),*>>,
            }

            impl<$t, $($ts),*> [<Notifier $cnt>]<$t, $($ts),*> {
                pub fn new() -> Self {
                    Self {
                        readys: vec![],
                    }
                }

                fn insert_handle(&mut self, handle: [<EventHandle $cnt>]<$t, $($ts),*>) {
                    self.readys.push(handle);
                }
            }
        }
    };
    {$cnt:expr,} => {
        pub struct Notifier {
            readys: Vec<EventHandle>,
        }

        impl Notifier {
            pub fn new() -> Self {
                Self {
                    readys: vec![],
                }
            }

            fn insert_handle(&mut self, handle: EventHandle) {
                self.readys.push(handle);
            }
        }
    };
}

decl_notifier! {8, TA TB TC TD TE TF TG TH}
decl_notifier! {7, TA TB TC TD TE TF TG}
decl_notifier! {6, TA TB TC TD TE TF}
decl_notifier! {5, TA TB TC TD TE}
decl_notifier! {4, TA TB TC TD}
decl_notifier! {3, TA TB TC}
decl_notifier! {2, TA TB}
decl_notifier! {1, TA}
decl_notifier! {0, }

/// Macro for helping implementing invoke method for various `Notifier` types.
macro_rules! notifier_impl_invoke {
    {$cnt:expr, $t:ident $($ts:ident)*, $i:ident $($is:ident)*} => {
        paste! {
            impl<$t, $($ts),*> [<Notifier $cnt>]<$t, $($ts),*>
            where
                $t: Copy,
                $($ts: Copy,)*
            {
                pub fn invoke(&self, $i: $t, $($is: $ts),*) {
                    for handle in &self.readys {
                        handle.call($i, $($is),*);
                    }
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl Notifier {
            pub fn invoke(&self) {
                for handle in &self.readys {
                    handle.call();
                }
            }
        }
    };
}

notifier_impl_invoke! {8, A B C D E F G H, a b c d e f g h}
notifier_impl_invoke! {7, A B C D E F G, a b c d e f g}
notifier_impl_invoke! {6, A B C D E F, a b c d e f}
notifier_impl_invoke! {5, A B C D E, a b c d e}
notifier_impl_invoke! {4, A B C D, a b c d}
notifier_impl_invoke! {3, A B C, a b c}
notifier_impl_invoke! {2, A B, a b}
notifier_impl_invoke! {1, A, a}
notifier_impl_invoke! {0, }

/// Macro for helping implementing internal functons for various `Notifier` types.
macro_rules! notifier_impl_internals {
    {$cnt:expr, $t:ident $($ts:ident)*} => {
        paste! {
            impl<$t, $($ts),*> [<Notifier $cnt>]<$t, $($ts),*>
            where
                $t: Copy + 'static,
                $($ts: Copy + 'static,)*
            {
                #[must_use]
                fn create_closure(f: impl Fn($t, $($ts),*) + Sync + Send + 'static,
                ) -> ([<Event $cnt>]<$t, $($ts),*>, [<EventHandle $cnt>]<$t, $($ts),*>) {
                    let event = [<Event $cnt>]::<$t, $($ts),*>::from_closure(f);
                    let handle = event.handle();
                    (event, handle)
                }

                #[must_use]
                fn create_method<TY, FN>(t: &TY, f: FN) ->
                    ([<Event $cnt>]<$t, $($ts),*>, [<EventHandle $cnt>]<$t, $($ts),*>)
                where
                    TY: 'static,
                    FN: Fn(&TY, $t, $($ts),*) + Sync + Send + 'static,
                {
                    let event = [<Event $cnt>]::<$t, $($ts),*>::from_method(t, f);
                    let handle = event.handle();
                    (event, handle)
                }

                #[must_use]
                fn create_method_mut<TY, FN>(t: &mut TY, f: FN) ->
                    ([<Event $cnt>]<$t, $($ts),*>, [<EventHandle $cnt>]<$t, $($ts),*>)
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $t, $($ts),*) + Sync + Send + 'static,
                {
                    let event = [<Event $cnt>]::<$t, $($ts),*>::from_method_mut(t, f);
                    let handle = event.handle();
                    (event, handle)
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl Notifier {
            #[must_use]
            fn create_closure(f: impl Fn() + Sync + Send + 'static) -> (Event, EventHandle) {
                let event = Event::from_closure(f);
                let handle = event.handle();
                (event, handle)
            }

            #[must_use]
            fn create_method<TY, FN>(t: &TY, f: FN) -> (Event, EventHandle)
            where
                TY: 'static,
                FN: Fn(&TY) + Sync + Send + 'static,
            {
                let event = Event::from_method(t, f);
                let handle = event.handle();
                (event, handle)
            }

            #[must_use]
            fn create_method_mut<TY, FN>(t: &mut TY, f: FN) -> (Event, EventHandle)
            where
                TY: 'static,
                FN: Fn(&mut TY) + Sync + Send + 'static,
            {
                let event = Event::from_method_mut(t, f);
                let handle = event.handle();
                (event, handle)
            }
        }
    };
}

/// Macro for helping implementing registration methods for various `Notifier` types.
macro_rules! notifier_impl_register {
    {$cnt:expr, $t:ident $($ts:ident)*} => {
        paste! {
            impl<$t, $($ts),*> [<Notifier $cnt>]<$t, $($ts),*>
            where
                $t: Copy + 'static,
                $($ts: Copy + 'static,)*
            {
                #[must_use]
                fn register_closure(
                    &mut self,
                    f: impl Fn($t, $($ts),*) + Sync + Send + 'static,
                ) -> [<Event $cnt>]<$t, $($ts),*> {
                    let (event, handle) = Self::create_closure(f);
                    self.insert_handle(handle);
                    event
                }

                #[must_use]
                pub fn register_method<TY, FN>(&mut self, t: &TY, f: FN) -> [<Event $cnt>]<$t, $($ts),*>
                where
                    TY: 'static,
                    FN: Fn(&TY, $t, $($ts),*) + Sync + Send + 'static,
                {
                    let (event, handle) = Self::create_method(t, f);
                    self.insert_handle(handle);
                    event
                }

                #[must_use]
                pub fn register_method_mut<TY, FN>(&mut self, t: &mut TY, f: FN) -> [<Event $cnt>]<$t, $($ts),*>
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $t, $($ts),*) + Sync + Send + 'static,
                {
                    let (event, handle) = Self::create_method_mut(t, f);
                    self.insert_handle(handle);
                    event
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl Notifier {
            #[must_use]
            pub fn register_closure(&mut self, f: impl Fn() + Sync + Send + 'static) -> Event {
                let (event, handle) = Self::create_closure(f);
                self.insert_handle(handle);
                event
            }

            #[must_use]
            pub fn register_method<TY, FN>(&mut self, t: &TY, f: FN) -> Event
            where
                TY: 'static,
                FN: Fn(&TY) + Sync + Send + 'static,
            {
                let (event, handle) = Self::create_method(t, f);
                self.insert_handle(handle);
                event
            }

            #[must_use]
            pub fn register_method_mut<TY, FN>(&mut self, t: &mut TY, f: FN) -> Event
            where
                TY: 'static,
                FN: Fn(&mut TY) + Sync + Send + 'static,
            {
                let (event, handle) = Self::create_method_mut(t, f);
                self.insert_handle(handle);
                event
            }
        }
    };
}

notifier_impl_internals! {8, TA TB TC TD TE TF TG TH}
notifier_impl_internals! {7, TA TB TC TD TE TF TG}
notifier_impl_internals! {6, TA TB TC TD TE TF}
notifier_impl_internals! {5, TA TB TC TD TE}
notifier_impl_internals! {4, TA TB TC TD}
notifier_impl_internals! {3, TA TB TC}
notifier_impl_internals! {2, TA TB}
notifier_impl_internals! {1, TA}
notifier_impl_internals! {0, }

notifier_impl_register! {8, TA TB TC TD TE TF TG TH}
notifier_impl_register! {7, TA TB TC TD TE TF TG}
notifier_impl_register! {6, TA TB TC TD TE TF}
notifier_impl_register! {5, TA TB TC TD TE}
notifier_impl_register! {4, TA TB TC TD}
notifier_impl_register! {3, TA TB TC}
notifier_impl_register! {2, TA TB}
notifier_impl_register! {1, TA}
notifier_impl_register! {0, }

#[cfg(test)]
mod test {
    use super::*;
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
        println!("Hello world!");

        let mut notifier = Notifier8::<Subject1, i32, i32, i32, i32, i32, i32, Subject1>::new();
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
}
