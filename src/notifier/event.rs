use paste::paste;
use std::{
    cell::RefCell,
    default::Default,
    marker::PhantomData,
    ptr::NonNull,
    sync::{Arc, Weak},
};

/// Macro for helping declaring functor traits which have different generic types and counts.
macro_rules! decl_functor {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            trait [<Functor $cnt>]<$($ts),*>: Sync + Send {
                fn call(&self, $(_: &'_ $ts),*);
            }
        }
    };
    {$cnt:expr,} => {
        trait Functor: Sync + Send {
            fn call(&self);
        }
    };
}

decl_functor! {8, TA TB TC TD TE TF TG TH}
decl_functor! {7, TA TB TC TD TE TF TG}
decl_functor! {6, TA TB TC TD TE TF}
decl_functor! {5, TA TB TC TD TE}
decl_functor! {4, TA TB TC TD}
decl_functor! {3, TA TB TC}
decl_functor! {2, TA TB}
decl_functor! {1, TA}
decl_functor! {0, }

/// Macro for helping declaring `PhantomData` wrapper
/// for avoiding inconstrated types in method types.
///
/// `PhantomWrapper` for parameter 0 count is not needed
/// because there is no types to contract as a struct generic types.
macro_rules! decl_phantom_wrapper {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            struct [<PhantomWrapper $cnt>]<$($ts),*> {
                $([<_ $is>]: PhantomData<$ts>,)*
            }

            impl<$($ts),*> Default for [<PhantomWrapper $cnt>]<$($ts),*> {
                fn default() -> Self {
                    Self {
                        $([<_ $is>]: PhantomData,)*
                    }
                }
            }
        }
    };
}

decl_phantom_wrapper! {8, A B C D E F G H, a b c d e f g h}
decl_phantom_wrapper! {7, A B C D E F G, a b c d e f g}
decl_phantom_wrapper! {6, A B C D E F, a b c d e f}
decl_phantom_wrapper! {5, A B C D E, a b c d e}
decl_phantom_wrapper! {4, A B C D, a b c d}
decl_phantom_wrapper! {3, A B C, a b c}
decl_phantom_wrapper! {2, A B, a b}
decl_phantom_wrapper! {1, A, a}

/// Low-level event type that contains arbitrary `'static` closure.
struct EventClosure<FN> {
    f: FN,
}

/// Macro for helping implementing functor traits to `EventClosure`.
macro_rules! event_closure_impl_functor {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<FN, $($ts),*> [<Functor $cnt>]<$($ts),*> for EventClosure<FN>
            where for<'any> FN: Fn($(&'any $ts),*) + Sync + Send,
            {
                fn call<'a>(&'a self, $($is: &'a $ts),*) { (self.f)($($is),*); }
            }
        }
    };
    {$cnt:expr,} => {
        impl<FN> Functor for EventClosure<FN> where FN: Fn() + Sync + Send,
        {
            fn call(&self) { (self.f)(); }
        }
    };
}

event_closure_impl_functor! {8, A B C D E F G H, a b c d e f g h}
event_closure_impl_functor! {7, A B C D E F G, a b c d e f g}
event_closure_impl_functor! {6, A B C D E F, a b c d e f}
event_closure_impl_functor! {5, A B C D E, a b c d e}
event_closure_impl_functor! {4, A B C D, a b c d}
event_closure_impl_functor! {3, A B C, a b c}
event_closure_impl_functor! {2, A B, a b}
event_closure_impl_functor! {1, A, a}
event_closure_impl_functor! {0, }

/// Macro for helping declaring `EventMethod` type which have various generic types.
macro_rules! decl_event_method {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            struct [<EventMethod $cnt>]<TY, FN, $($ts),*> {
                t: NonNull<TY>,
                f: FN,
                _phantom: [<PhantomWrapper $cnt>]<$($ts),*>,
            }
            unsafe impl<TY, FN, $($ts),*> Sync for [<EventMethod $cnt>]<TY, FN, $($ts),*> where FN: Fn(&TY, $(&'_ $ts),*) + Sync + Send {}
            unsafe impl<TY, FN, $($ts),*> Send for [<EventMethod $cnt>]<TY, FN, $($ts),*> where FN: Fn(&TY, $(&'_ $ts),*) + Sync + Send {}
        }
    };
    {$cnt:expr,} => {
        struct EventMethod<TY, FN> {
            t: NonNull<TY>,
            f: FN,
        }
        unsafe impl<TY, FN> Sync for EventMethod<TY, FN> where FN: Fn(&TY) + Sync + Send {}
        unsafe impl<TY, FN> Send for EventMethod<TY, FN> where FN: Fn(&TY) + Sync + Send {}
    };
}

decl_event_method! {8, TA TB TC TD TE TF TG TH}
decl_event_method! {7, TA TB TC TD TE TF TG}
decl_event_method! {6, TA TB TC TD TE TF}
decl_event_method! {5, TA TB TC TD TE}
decl_event_method! {4, TA TB TC TD}
decl_event_method! {3, TA TB TC}
decl_event_method! {2, TA TB}
decl_event_method! {1, TA}
decl_event_method! {0, }

/// Macro for helping implementing generic `Functor` traits to various `EventMethod` types.
macro_rules! event_method_impl_functor {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<TY, FN, $($ts),*> [<Functor $cnt>]<$($ts),*> for [<EventMethod $cnt>]<TY, FN, $($ts),*>
            where FN: Fn(&'_ TY, $(&'_ $ts),*) + Sync + Send,
            {
                fn call<'a>(&'a self, $($is: &'a $ts),*) {
                    (self.f)(unsafe { self.t.as_ref() }, $($is),*);
                }
            }

            impl<TY, FN, $($ts),*> [<EventMethod $cnt>]<TY, FN, $($ts),*>
            where FN: Fn(&'_ TY, $(&'_ $ts),*) + Sync + Send,
            {
                fn new(t: NonNull<TY>, f: FN) -> Self {
                    Self {
                        t, f, _phantom: [<PhantomWrapper $cnt>]::default()
                    }
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl<TY, FN> Functor for EventMethod<TY, FN>
            where FN: Fn(&TY) + Sync + Send,
        {
            fn call(&self) {
                (self.f)(unsafe { self.t.as_ref() });
            }
        }

        impl<TY, FN> EventMethod<TY, FN>
            where FN: Fn(&TY) + Sync + Send,
        {
            fn new(t: NonNull<TY>, f: FN) -> Self { Self { t, f } }
        }
    };
}

event_method_impl_functor! {8, A B C D E F G H, a b c d e f g h}
event_method_impl_functor! {7, A B C D E F G, a b c d e f g}
event_method_impl_functor! {6, A B C D E F, a b c d e f}
event_method_impl_functor! {5, A B C D E, a b c d e}
event_method_impl_functor! {4, A B C D, a b c d}
event_method_impl_functor! {3, A B C, a b c}
event_method_impl_functor! {2, A B, a b}
event_method_impl_functor! {1, A, a}
event_method_impl_functor! {0, }

/// Macro for helping declaring `EventMethodMut` type which have various generic types.
macro_rules! decl_event_methodmut {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            struct [<EventMethodMut $cnt>]<TY, FN, $($ts),*> {
                t: RefCell<NonNull<TY>>,
                f: FN,
                _phantom: [<PhantomWrapper $cnt>]<$($ts),*>,
            }
            unsafe impl<TY, FN, $($ts),*> Sync for [<EventMethodMut $cnt>]<TY, FN, $($ts),*> where FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send {}
            unsafe impl<TY, FN, $($ts),*> Send for [<EventMethodMut $cnt>]<TY, FN, $($ts),*> where FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send {}

            impl<TY, FN, $($ts),*> [<EventMethodMut $cnt>]<TY, FN, $($ts),*>
            where FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send,
            {
                fn new(t: RefCell<NonNull<TY>>, f: FN) -> Self {
                    Self {
                        t, f, _phantom: [<PhantomWrapper $cnt>]::default()
                    }
                }
            }
        }
    };
    {$cnt:expr,} => {
        struct EventMethodMut<TY, FN> {
            t: RefCell<NonNull<TY>>,
            f: FN,
        }
        unsafe impl<TY, FN> Sync for EventMethodMut<TY, FN> where FN: Fn(&mut TY) + Sync + Send {}
        unsafe impl<TY, FN> Send for EventMethodMut<TY, FN> where FN: Fn(&mut TY) + Sync + Send {}

        impl<TY, FN> EventMethodMut<TY, FN>
            where FN: Fn(&mut TY) + Sync + Send,
        {
            fn new(t: RefCell<NonNull<TY>>, f: FN) -> Self { Self { t, f } }
        }
    };
}

decl_event_methodmut! {8, TA TB TC TD TE TF TG TH}
decl_event_methodmut! {7, TA TB TC TD TE TF TG}
decl_event_methodmut! {6, TA TB TC TD TE TF}
decl_event_methodmut! {5, TA TB TC TD TE}
decl_event_methodmut! {4, TA TB TC TD}
decl_event_methodmut! {3, TA TB TC}
decl_event_methodmut! {2, TA TB}
decl_event_methodmut! {1, TA}
decl_event_methodmut! {0, }

/// Macro for helping implementing generic `Functor` traits to various `EventMethodMut` types.
macro_rules! event_methodmut_impl_functor {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<TY, FN, $($ts),*> [<Functor $cnt>]<$($ts),*> for [<EventMethodMut $cnt>]<TY, FN, $($ts),*>
            where FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send,
            {
                fn call<'a>(&'a self, $($is: &'a $ts),*) {
                    (self.f)(unsafe { self.t.borrow_mut().as_mut() }, $($is),*);
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl<TY, FN> Functor for EventMethodMut<TY, FN> where FN: Fn(&mut TY) + Sync + Send,
        {
            fn call(&self) {
                (self.f)(unsafe { self.t.borrow_mut().as_mut() });
            }
        }
    };
}

event_methodmut_impl_functor! {8, A B C D E F G H, a b c d e f g h}
event_methodmut_impl_functor! {7, A B C D E F G, a b c d e f g}
event_methodmut_impl_functor! {6, A B C D E F, a b c d e f}
event_methodmut_impl_functor! {5, A B C D E, a b c d e}
event_methodmut_impl_functor! {4, A B C D, a b c d}
event_methodmut_impl_functor! {3, A B C, a b c}
event_methodmut_impl_functor! {2, A B, a b}
event_methodmut_impl_functor! {1, A, a}
event_methodmut_impl_functor! {0, }

/// Macro for helping declaring `EventRaw` type which have various generic types.
macro_rules! decl_event_raw {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            struct [<EventRaw $cnt>]<$($ts),*> {
                func: Box<dyn [<Functor $cnt>]<$($ts),*>>,
            }
        }
    };
    {$cnt:expr,} => {
        struct EventRaw<> {
            func: Box<dyn Functor>,
        }
    };
}

decl_event_raw! {8, TA TB TC TD TE TF TG TH}
decl_event_raw! {7, TA TB TC TD TE TF TG}
decl_event_raw! {6, TA TB TC TD TE TF}
decl_event_raw! {5, TA TB TC TD TE}
decl_event_raw! {4, TA TB TC TD}
decl_event_raw! {3, TA TB TC}
decl_event_raw! {2, TA TB}
decl_event_raw! {1, TA}
decl_event_raw! {0, }

/// Macro for helping implementing methods for various `EventRaw` types.
macro_rules! event_raw_impl_call {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<$($ts),*> [<EventRaw $cnt>]<$($ts),*> {
                fn call<'a>(&'a self, $($is: &'a $ts),*) {
                    self.func.call($($is),*);
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl EventRaw {
            fn call(&self) {
                self.func.call();
            }
        }
    };
}

event_raw_impl_call! {8, A B C D E F G H, a b c d e f g h}
event_raw_impl_call! {7, A B C D E F G, a b c d e f g}
event_raw_impl_call! {6, A B C D E F, a b c d e f}
event_raw_impl_call! {5, A B C D E, a b c d e}
event_raw_impl_call! {4, A B C D, a b c d}
event_raw_impl_call! {3, A B C, a b c}
event_raw_impl_call! {2, A B, a b}
event_raw_impl_call! {1, A, a}
event_raw_impl_call! {0, }

/// Macro for helping implementing methods for various `EventRaw` types.
macro_rules! event_raw_impl_from {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            impl<$($ts),*> [<EventRaw $cnt>]<$($ts),*>
            where $($ts: 'static),*
            {
                fn from_closure<FN>(f: FN) -> Self
                where
                    FN: Fn($(&'_ $ts),*) + Sync + Send + 'static,
                {
                    Self {
                        func: Box::new(EventClosure { f }),
                    }
                }

                fn from_method<TY, FN>(t: &TY, f: FN) -> Self
                where
                    TY: 'static,
                    FN: Fn(&TY, $(&'_ $ts),*) + Sync + Send + 'static,
                {
                    let t = NonNull::new(t as *const _ as *mut TY).unwrap();
                    let i = [<EventMethod $cnt>]::<TY, FN, $($ts),*>::new(t, f);
                    Self { func: Box::new(i) }
                }

                fn from_method_mut<TY, FN>(t: &mut TY, f: FN) -> Self
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send + 'static,
                {
                    let t = RefCell::new(NonNull::new(t as *mut TY).unwrap());
                    let i = [<EventMethodMut $cnt>]::<TY, FN, $($ts),*>::new(t, f);
                    Self { func: Box::new(i) }
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl EventRaw {
            fn from_closure<FN>(f: FN) -> Self
            where
                FN: Fn() + Sync + Send + 'static,
            {
                Self {
                    func: Box::new(EventClosure { f }),
                }
            }

            fn from_method<TY, FN>(t: &TY, f: FN) -> Self
            where
                TY: 'static,
                FN: Fn(&TY) + Sync + Send + 'static,
            {
                let t = NonNull::new(t as *const _ as *mut TY).unwrap();
                let i = EventMethod::<TY, FN>::new(t, f);
                Self { func: Box::new(i) }
            }

            fn from_method_mut<TY, FN>(t: &mut TY, f: FN) -> Self
            where
                TY: 'static,
                FN: Fn(&mut TY) + Sync + Send + 'static,
            {
                let t = RefCell::new(NonNull::new(t as *mut TY).unwrap());
                let i = EventMethodMut::<TY, FN>::new(t, f);
                Self { func: Box::new(i) }
            }
        }
    };
}

event_raw_impl_from! {8, A B C D E F G H}
event_raw_impl_from! {7, A B C D E F G}
event_raw_impl_from! {6, A B C D E F}
event_raw_impl_from! {5, A B C D E}
event_raw_impl_from! {4, A B C D}
event_raw_impl_from! {3, A B C}
event_raw_impl_from! {2, A B}
event_raw_impl_from! {1, A}
event_raw_impl_from! {0,}

/// Macro for helping declaring `EventHandle` type which have various generic types.
macro_rules! decl_event_handle {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            pub(super) struct [<EventHandle $cnt>]<$($ts),*> {
                raw: Weak<[<EventRaw $cnt>]<$($ts),*>>,
            }
        }
    };
    {$cnt:expr,} => {
        pub(super) struct EventHandle {
            raw: Weak<EventRaw>,
        }
    };
}

decl_event_handle! {8, TA TB TC TD TE TF TG TH}
decl_event_handle! {7, TA TB TC TD TE TF TG}
decl_event_handle! {6, TA TB TC TD TE TF}
decl_event_handle! {5, TA TB TC TD TE}
decl_event_handle! {4, TA TB TC TD}
decl_event_handle! {3, TA TB TC}
decl_event_handle! {2, TA TB}
decl_event_handle! {1, TA}
decl_event_handle! {0, }

/// Macro for helping implementing methods for various `EventHandle` types.
macro_rules! event_handle_impl_call {
    {$cnt:expr, $($ts:ident) +, $($is:ident) +} => {
        paste! {
            impl<$($ts),*> [<EventHandle $cnt>]<$($ts),*> {
                pub(super) fn call<'a>(&'a self, $($is: &'a $ts),*) {
                    if let Some(raw) = self.raw.upgrade() {
                        raw.call($($is),*);
                    }
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl EventHandle {
            pub(super) fn call(&self) {
                if let Some(raw) = self.raw.upgrade() {
                    raw.call();
                }
            }
        }
    };
}

event_handle_impl_call! {8, A B C D E F G H, a b c d e f g h}
event_handle_impl_call! {7, A B C D E F G, a b c d e f g}
event_handle_impl_call! {6, A B C D E F, a b c d e f}
event_handle_impl_call! {5, A B C D E, a b c d e}
event_handle_impl_call! {4, A B C D, a b c d}
event_handle_impl_call! {3, A B C, a b c}
event_handle_impl_call! {2, A B, a b}
event_handle_impl_call! {1, A, a}
event_handle_impl_call! {0, }

/// Macro for helping declaring `Event` type which have various generic types.
macro_rules! decl_event {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            pub struct [<Event $cnt>]<$($ts),*> {
                raw: Arc<[<EventRaw $cnt>]<$($ts),*>>,
            }
        }
    };
    {$cnt:expr,} => {
        pub struct Event {
            raw: Arc<EventRaw>,
        }
    };
}

decl_event! {8, TA TB TC TD TE TF TG TH}
decl_event! {7, TA TB TC TD TE TF TG}
decl_event! {6, TA TB TC TD TE TF}
decl_event! {5, TA TB TC TD TE}
decl_event! {4, TA TB TC TD}
decl_event! {3, TA TB TC}
decl_event! {2, TA TB}
decl_event! {1, TA}
decl_event! {0, }

/// Macro for helping implementing methods for various `Event` types.
macro_rules! event_impl_from {
    {$cnt:expr, $($ts:ident) +} => {
        paste! {
            impl<$($ts),*> [<Event $cnt>]<$($ts),*>
            where $($ts: 'static),*
            {
                pub(super) fn from_closure<FN>(f: FN) -> Self
                where
                    FN: Fn($(&'_ $ts),*) + Sync + Send + 'static,
                {
                    let raw = [<EventRaw $cnt>]::<$($ts),*>::from_closure(f);
                    Self { raw: Arc::new(raw) }
                }

                pub(super) fn from_method<TY, FN>(t: &TY, f: FN) -> Self
                where
                    TY: 'static,
                    FN: Fn(&TY, $(&'_ $ts),*) + Sync + Send + 'static,
                {
                    let raw = [<EventRaw $cnt>]::<$($ts),*>::from_method(t, f);
                    Self { raw: Arc::new(raw) }
                }

                pub(super) fn from_method_mut<TY, FN>(t: &mut TY, f: FN) -> Self
                where
                    TY: 'static,
                    FN: Fn(&mut TY, $(&'_ $ts),*) + Sync + Send + 'static,
                {
                    let raw = [<EventRaw $cnt>]::<$($ts),*>::from_method_mut(t, f);
                    Self { raw: Arc::new(raw) }
                }
            }

            impl<$($ts),*> [<Event $cnt>]<$($ts),*> {
                pub(super) fn handle(&self) -> [<EventHandle $cnt>]<$($ts),*> {
                    [<EventHandle $cnt>]::<$($ts),*> {
                        raw: Arc::downgrade(&self.raw),
                    }
                }
            }
        }
    };
    {$cnt:expr,} => {
        impl Event {
            pub(super) fn from_closure<FN>(f: FN) -> Self
            where
                FN: Fn() + Sync + Send + 'static,
            {
                Self { raw: Arc::new(EventRaw::from_closure(f)) }
            }

            pub(super) fn from_method<TY, FN>(t: &TY, f: FN) -> Self
            where
                TY: 'static,
                FN: Fn(&TY) + Sync + Send + 'static,
            {
                Self { raw: Arc::new(EventRaw::from_method(t, f)) }
            }

            pub(super) fn from_method_mut<TY, FN>(t: &mut TY, f: FN) -> Self
            where
                TY: 'static,
                FN: Fn(&mut TY) + Sync + Send + 'static,
            {
                Self { raw: Arc::new(EventRaw::from_method_mut(t, f)) }
            }
        }

        impl Event {
            pub(super) fn handle(&self) -> EventHandle{
                EventHandle {
                    raw: Arc::downgrade(&self.raw),
                }
            }
        }
    };
}

event_impl_from! {8, TA TB TC TD TE TF TG TH}
event_impl_from! {7, TA TB TC TD TE TF TG}
event_impl_from! {6, TA TB TC TD TE TF}
event_impl_from! {5, TA TB TC TD TE}
event_impl_from! {4, TA TB TC TD}
event_impl_from! {3, TA TB TC}
event_impl_from! {2, TA TB}
event_impl_from! {1, TA}
event_impl_from! {0, }
