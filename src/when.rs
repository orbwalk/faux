//! Tools to stub the implementation or return value of your mocks.

mod once;

use crate::{
    matcher::{AnyInvocation, InvocationMatcher},
    mock_store::MockStore,
    stub::{self, Stub},
};
pub use once::Once;

/// Provides methods to stub the implementation or return value of the
/// stubbed method.
///
/// Created using [`when!`].
///
/// By default, methods are stubbed for all invocations. Use [`when!`]
/// for an ergonomic way to set argument matchers. For more features,
/// use [`with_args`].
///
/// By default, all methods are stubbed indefinitely. Thus, any
/// stubbed values needs to be cloneable and any stubbed
/// implementation cannot consume variables. Use the [`times`] and
/// [`once`] methods to override these defaults.
///
/// Do *NOT* rely on the signature of `When`. While changing the
/// methods of `When` will be considered a breaking change, changing
/// the generics within `When` will not.
///
/// [`when!`]: crate::when!
/// [`once`]: When::once
/// [`times`]: When::times
/// [`with_args`]: When::with_args
pub struct When<'q, R, I, O, M: InvocationMatcher<I, N>, const N: usize> {
    id: fn(R, I) -> O,
    store: &'q mut MockStore,
    times: stub::Times,
    matcher: M,
}

impl<'q, R, I, O, const N: usize> When<'q, R, I, O, AnyInvocation, N> {
    #[doc(hidden)]
    pub fn new(id: fn(R, I) -> O, store: &'q mut MockStore) -> Self {
        When {
            id,
            store,
            matcher: AnyInvocation,
            times: stub::Times::Always,
        }
    }
}

impl<'q, R, I, O, M: InvocationMatcher<I, N> + Send + 'static, const N: usize>
    When<'q, R, I, O, M, N>
{
    /// Sets the return value of the stubbed method.
    ///
    /// Requires the value to be static. For a more lax but unsafe
    /// alternative, use [`then_unchecked_return`].
    ///
    /// The returned value will be cloned on each invocation of the
    /// stub. Using this method to return another mock instance will
    /// fail unless that stub instance has its clone method already
    /// stubbed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn multi_args(&mut self, a: &i32, b: i8) -> u32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   faux::when!(mock.multi_args).then_return(5);
    ///   // mock activates multiple times
    ///   assert_eq!(mock.multi_args(&2, 3), 5);
    ///   assert_eq!(mock.multi_args(&2, 3), 5);
    /// }
    /// ```
    ///
    /// [`then_unchecked_return`]: When::then_unchecked_return
    pub fn then_return(self, value: O)
    where
        O: Send + Clone + 'static,
    {
        self.then(move |_: I| value.clone());
    }

    /// Sets the implementation of the stubbed method to the provided
    /// closure.
    ///
    /// The input to the closure is a tuple of all its non-receiver
    /// parameters.
    ///
    /// The provided closure can only capture static variables and it
    /// must be stubbing a method with static output. For a more lax
    /// but unsafe alternative, use [`then_unchecked`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn no_args(&mut self) -> i32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    ///
    ///     pub fn single_arg(&self, a: u8) -> Vec<i8> {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    ///
    ///     pub fn multi_args(self, a: &i32, b: i8) -> u32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    ///
    ///     pub fn out_ref(&self) -> &u32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   // method with no params
    ///   faux::when!(mock.no_args).then(|_| 5);
    ///   assert_eq!(mock.no_args(), 5);
    ///
    ///   // method with a single param
    ///   faux::when!(mock.single_arg).then(|input| vec![input as i8]);
    ///   assert_eq!(mock.single_arg(8), vec![8]);
    ///
    ///   // method with multiple params - some can be references
    ///   faux::when!(mock.multi_args).then(|(&a, _)| a as u32);
    ///   assert_eq!(mock.multi_args(&5, 2), 5);
    ///
    ///   // cannot stub methods that return references
    ///   // let x = 5;
    ///   // faux::when!(mock.out_ref).then(|_| &x);
    /// }
    /// ```
    ///
    /// [`then_unchecked`]: When::then_unchecked
    pub fn then(self, stub: impl FnMut(I) -> O + 'static + Send)
    where
        O: 'static,
    {
        self.store.stub(
            self.id,
            Stub::new(
                stub::Answer::Many {
                    times: self.times,
                    stub: Box::new(stub),
                },
                self.matcher,
            ),
        );
    }

    /// Analog of [`then_return`] that allows stubbing non-static
    /// return values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn multi_args(&mut self, a: &i32, b: i8) -> &u32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   let x = 5;
    ///   unsafe { faux::when!(mock.multi_args).then_unchecked_return(&x) }
    ///   assert_eq!(*mock.multi_args(&2, 3), x);
    /// }
    /// ```
    ///
    /// # Safety
    ///
    /// The lifetime of the returned object is not checked and can
    /// cause memory safety issues if used incorrectly.
    ///
    /// If the owner of the borrowed data is dropped while the
    /// captured reference is still accessible, a use-after-free
    /// violation will be triggered.
    ///
    /// This method can also cause aliasing issues where multiple
    /// mutable references are held for the same object.
    ///
    /// ### Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn out_ref(&self) -> &u32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   let x = 5;
    ///   unsafe { faux::when!(mock.out_ref).then_unchecked_return(&x) }
    ///   std::mem::drop(x);
    ///   // assert_eq!(*mock.ref_return(), 5); // <~~ UB: use after free
    /// }
    /// ```
    ///
    /// [`then_return`]: When::then_return
    pub unsafe fn then_unchecked_return(self, value: O)
    where
        O: Send + Clone,
    {
        self.then_unchecked(move |_: I| value.clone())
    }

    /// Analog of [`then`] that allows stubbing implementations with
    /// non-static closures.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn out_ref(&mut self) -> &i32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   // the output can be a reference to the environment
    ///   // but this can be *very* dangerous
    ///   let x = 5;
    ///   unsafe { faux::when!(mock.out_ref).then_unchecked(|_| &x) }
    ///   assert_eq!(*mock.out_ref(), x);
    /// }
    ///
    /// ```
    ///
    ///
    /// # Safety
    ///
    /// The lifetimes of the outputs and captured variables are not
    /// checked. While this gives the caller maximum flexibility when
    /// mocking, it is *not* memory safe when used incorrectly.
    ///
    /// If the mocked method is called after its captured variables
    /// are dropped then a use-after-free violation will be triggered.
    ///
    /// Relationships between inputs, outputs, and captured variable
    /// lifetimes are lost. This allows for easy violations of Rust's
    /// aliasing checks, creating undefined behavior.
    ///
    /// ### Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn out_ref(&self, a : &mut i32) -> &mut i32 {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///   // the output and input references are the same
    ///   unsafe { faux::when!(mock.out_ref).then_unchecked(|i| i) }
    ///
    ///   let mut x = 5;
    ///   // y (the output) is a mutable reference back to x (the input)
    ///   // but there is no compile-time link between the two
    ///   let y = mock.out_ref(&mut x);
    ///
    ///   // x and y are pointing to the same data!
    ///   assert_eq!(*y, 5);
    ///   assert_eq!(x, 5);
    ///
    ///   // changes in x are reflected in y and vice versa
    ///   // this is UB and is not allowed in safe Rust!
    ///   x += 1;
    ///   assert_eq!(x, 6);
    ///   assert_eq!(*y, 6);
    /// }
    /// ```
    ///
    /// [`then`]: When::then
    pub unsafe fn then_unchecked(self, mock: impl FnMut(I) -> O + Send) {
        self.store.stub_unchecked(
            self.id,
            Stub::new(
                stub::Answer::Many {
                    times: self.times,
                    stub: Box::new(mock),
                },
                self.matcher,
            ),
        );
    }

    /// Limits the number of calls for which a mock is active.
    ///
    /// Calls past the limit will result in a panic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn single_arg(&self, a: u8) -> Vec<i8> {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   // limit to 5 calls
    ///   faux::when!(mock.single_arg)
    ///       .times(5)
    ///       .then(|input| vec![input as i8]);
    ///
    ///   // can be called 5 times
    ///   for _ in 0..5 {
    ///     assert_eq!(mock.single_arg(8), vec![8]);
    ///   }
    /// }
    /// ```
    ///
    /// ## Panics
    ///
    /// Panics if the mock is called more times than specified.
    ///
    /// ```rust should_panic
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn single_arg(&self, a: u8) -> Vec<i8> {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   // limit to 5 calls
    ///   faux::when!(mock.single_arg)
    ///       .times(5)
    ///       .then(|input| vec![input as i8]);
    ///
    ///   // panics on the 6th call
    ///   for _ in 0..6 {
    ///     assert_eq!(mock.single_arg(8), vec![8]);
    ///   }
    /// }
    /// ```
    pub fn times(mut self, times: usize) -> Self {
        self.times = stub::Times::Times(times);
        self
    }

    /// Limits mock to one call, allowing mocks to consume captured variables.
    ///
    /// Panics if the mock is called more than once.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn single_arg(&self, a: u8) -> Vec<i8> {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   let vec = vec![25];
    ///   // moves vec to the closure
    ///   faux::when!(mock.single_arg).once().then(|_| vec);
    ///   assert_eq!(mock.single_arg(8), vec![25]);
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the mock is called more than once.
    ///
    /// ```rust should_panic
    /// #[faux::create]
    /// pub struct Foo {}
    ///
    /// #[faux::methods]
    /// impl Foo {
    ///     pub fn single_arg(&self, a: u8) -> Vec<i8> {
    ///       /* implementation code */
    ///       # panic!()
    ///     }
    /// }
    ///
    /// fn main() {
    ///   let mut mock = Foo::faux();
    ///
    ///   let vec = vec![25];
    ///   faux::when!(mock.single_arg).once().then(|_| vec);
    ///   assert_eq!(mock.single_arg(8), vec![25]);
    ///   //panics on its second call
    ///   mock.single_arg(8);
    /// }
    /// ```
    pub fn once(self) -> Once<'q, R, I, O, M, N> {
        Once::new(self.id, self.store, self.matcher)
    }

    /// Specifies a matcher for the invocation.
    ///
    /// This lets you pass matchers for each method argument.
    ///
    /// See [`when!`](crate::when!) for an ergonomic way to pass the
    /// matcher.
    ///
    /// If all arguments implement [`Debug`](std::fmt::Debug), a tuple
    /// of [`ArgMatcher`](crate::ArgMatcher)s can be provided where
    /// each `ArgMatcher` matches an individual argument.
    ///
    /// If the method only has a single argument, use a tuple of a
    /// single element: `(ArgMatcher,)`
    ///
    /// For more complex cases, you may pass a custom
    /// [`InvocationMatcher`](InvocationMatcher).
    pub fn with_args<M2: InvocationMatcher<I, N> + Send + 'static>(
        self,
        matcher: M2,
    ) -> When<'q, R, I, O, M2, N> {
        When {
            matcher,
            times: self.times,
            id: self.id,
            store: self.store,
        }
    }
}
