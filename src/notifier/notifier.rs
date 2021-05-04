use super::event::*;
use paste::paste;

/// Macro for helping declaring `Notifier` type which have various generic types and some methods.
macro_rules! decl_notifier {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            pub struct [<Notifier $cnt>]<$($ts),*> {
                readys: Vec<[<EventHandle $cnt>]<$($ts),*>>,
            }

            impl<$($ts),*> [<Notifier $cnt>]<$($ts),*> {
                pub fn new() -> Self {
                    Self {
                        readys: vec![],
                    }
                }

                fn insert_handle(&mut self, handle: [<EventHandle $cnt>]<$($ts),*>) {
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
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<$($ts),*> [<Notifier $cnt>]<$($ts),*>
            where $($ts: Copy,)*
            {
                pub fn invoke(&self, $($is: $ts),*) {
                    for handle in &self.readys {
                        handle.call($($is),*);
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
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            impl<$($ts),*> [<Notifier $cnt>]<$($ts),*>
            where $($ts: Copy + 'static,)*
            {
                #[must_use]
                fn create_closure(f: impl Fn($($ts),*) + Sync + Send + 'static,
                ) -> ([<Event $cnt>]<$($ts),*>, [<EventHandle $cnt>]<$($ts),*>) {
                    let event = [<Event $cnt>]::<$($ts),*>::from_closure(f);
                    let handle = event.handle();
                    (event, handle)
                }

                #[must_use]
                fn create_method<TY, FN>(t: &TY, f: FN) ->
                    ([<Event $cnt>]<$($ts),*>, [<EventHandle $cnt>]<$($ts),*>)
                where
                    TY: 'static,
                    FN: Fn(&TY, $($ts),*) + Sync + Send + 'static,
                {
                    let event = [<Event $cnt>]::<$($ts),*>::from_method(t, f);
                    let handle = event.handle();
                    (event, handle)
                }

                #[must_use]
                fn create_method_mut<TY, FN>(t: &mut TY, f: FN) ->
                    ([<Event $cnt>]<$($ts),*>, [<EventHandle $cnt>]<$($ts),*>)
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $($ts),*) + Sync + Send + 'static,
                {
                    let event = [<Event $cnt>]::<$($ts),*>::from_method_mut(t, f);
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
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            impl<$($ts),*> [<Notifier $cnt>]<$($ts),*>
            where $($ts: Copy + 'static,)*
            {
                #[must_use]
                pub fn register_closure(
                    &mut self,
                    f: impl Fn($($ts),*) + Sync + Send + 'static,
                ) -> [<Event $cnt>]<$($ts),*> {
                    let (event, handle) = Self::create_closure(f);
                    self.insert_handle(handle);
                    event
                }

                #[must_use]
                pub fn register_method<TY, FN>(&mut self, t: &TY, f: FN) -> [<Event $cnt>]<$($ts),*>
                where
                    TY: 'static,
                    FN: Fn(&TY, $($ts),*) + Sync + Send + 'static,
                {
                    let (event, handle) = Self::create_method(t, f);
                    self.insert_handle(handle);
                    event
                }

                #[must_use]
                pub fn register_method_mut<TY, FN>(&mut self, t: &mut TY, f: FN) -> [<Event $cnt>]<$($ts),*>
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $($ts),*) + Sync + Send + 'static,
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
