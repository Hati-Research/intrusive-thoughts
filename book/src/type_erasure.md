# Type erasure in Rust - Manually creating fat pointers

> Note: This chapter is not that relevant for bare metal embedded development,
but the principles could be used there as well.

## Introduction

A common way of using type erasure in Rust is to define the shared behavior
using a trait and then to box a trait object, e.g. `let a: Box<dyn ATrait>  = Box::new(A)`.
There, however, is a **subtle and problematic** problem to it that
the type of `a` is actually `Box<dyn ATrait + 'static`.
Why would that be a problem?
Given that this detail is `hidden`, you may want to write a function that erases the type like this:


```rust
fn erase<T: ATrait>(value: T) -> Box<dyn ATrait> {
    Box::new(value)
}
```

However, when you try to compile it you hit a problem that the return type
requires more specialized `T` than the generic parameter, because the actual `'static` requirement was hidden.


```rust
error[E0310]: the parameter type `T` may not live long enough
  --> src/main.rs:19:5
   |
19 |     Box::new(value)
   |     ^^^^^^^^^^^^^^^
   |     |
   |     the parameter type `T` must be valid for the static lifetime...
   |     ...so that the type `T` will meet its required lifetime bounds
   |
help: consider adding an explicit lifetime bound
   |
18 | fn erase<T: ATrait + 'static>(value: T) -> Box<dyn ATrait> {
   |                    +++++++++

For more information about this error, try `rustc --explain E0310`.
```

## Why is this a problem? Just add `'static`

Well, in a greenfield project where you stumble upon this, this may well be the way to go. 
However, in a large codebase which didn't count on this requirement beforehand, this subtle change would mean a lot of refactoring.
This simple change would mean having to add `'static` everywhere this is used and usually generics are done with more than one level of indirection.
Yes, it could be argued that an API like this was poorly designed and adding `'static` is the correct way to go, but we have to work with what we get.


### Real world example - the `safer_ffi` crate
The `safer_ffi` crate is a crate used for generating code used in FFI (Foreign Function Interface) - code that can be used from other lnaguages, or when using other languages with Rust.

The problem is as follows:

1. There is a trait `CType` which is defines some metadata about a type in C's type system. [link](https://docs.rs/safer-ffi/latest/safer_ffi/layout/trait.CType.html)
2. Since in `CType` we only care about its methods without Self receiver, therefore we only need to access the type, not a value implementing `CType`.
3. The `CType` implementing types are then contained in a `Layout<T: CType>` struct defining its memory layout. There are more layout types, we will implement the `NonNullLayout<T: CType>`.
This Layout is also a CType and this is the place where we encounter a generic type which is not constrained by `'static`.
4. The `CType` trait has a method `fn metadata() -> &'static dyn Provider {}` which provides metadata about a type without the knowledge of the type - this is the type erased part.
5. We now want to add metadata for better cross language codegen. Let's define them using an enum `MetadataTypeUsage`.
For demonstration, we only care about the `NonNull` type of metadata, which indirects to the inner type.

```rust
enum MetadataTypeUsage {
  NonNull { inner: Box<dyn CType> }
}
```

6. The `Provider` trait is not important for this use case, let's define it as a trait capable of retrieving the metadata as follows.
7. If we were to compile this we'd find out that we cannot use `CType` directly as a trait object.

``` rust
error[E0038]: the trait `CType` is not dyn compatible
  --> src/main.rs:26:27
   |
26 | fn object_unsafe() -> Box<dyn CType> {
   |                           ^^^^^^^^^ `CType` is not dyn compatible
   |
note: for a trait to be dyn compatible it needs to allow building a vtable
      for more information, visit <https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility>
  --> src/main.rs:23:8
   |
22 | trait CType {
   |       ----- this trait is not dyn compatible...
23 |     fn metadata();
   |        ^^^^^^^^ ...because associated function `metadata` has no `self` parameter
help: consider turning `metadata` into a method by giving it a `&self` argument
   |
23 |     fn metadata(&self);
   |                 +++++
help: alternatively, consider constraining `metadata` so it does not apply to trait objects
   |
23 |     fn metadata() where Self: Sized;
   |                   +++++++++++++++++
```

8. To type erase a type implementing `CType`, we need to be able to access the type itself by wrapping into `PhantomData` and also, we need to define a new trait `PhantomCType` which will be object(dyn) safe. The `CType` is not object safe because it has methods without a `Self` receiver. ** This also forces us to change the signature of the `NonNull` enum variant.

To sum up with a minimal example.

```rust
trait CType {
    fn metadata() -> &'static dyn Provider;
}

trait PhantomCType {
    fn metadata(&self) -> &'static dyn Provider;
}

impl<T: CType> PhantomCType for PhantomData<T> {
    fn metadata(&self) -> &'static dyn Provider {
        T::metadata()
    }
}

struct NonNullCLayout<T: CType> {
    _phantom: PhantomData<T>,
}

impl<T: CType> CType for NonNullCLayout<T> {
    fn metadata() -> &'static dyn Provider {
        let usage = MetadataTypeUsage::NonNull {
            inner: Box::new(PhantomData::<T>),
        };
        let provider = Box::new(ProviderWrapper(usage));
        Box::leak(provider)
    }
}

enum MetadataTypeUsage {
    NonNull { inner: Box<dyn PhantomCType> },
}

trait Provider {
    fn provide(&self) -> &MetadataTypeUsage;
}

struct ProviderWrapper(MetadataTypeUsage);

impl Provider for ProviderWrapper {
    fn provide(&self) -> &MetadataTypeUsage {
        &self.0
    }
}
```

This finally brings us to the original problem with type erasure of a type which is not `'static`constrained.

### The solution: Create the Fat pointer directly

The solution is to go to the first principles of what a `dyn Trait` construct is
- a Fat pointer, meaning a pointer that is composed of a data pointer and
a VTable to methods that can be called on the data.
This basically means that we are able to create our own, if we are able to construct the VTable ourselves.
In this special case of `simpler_ffi`'s we won't even need the data pointer - just the VTable. and for this VTable struct, we can implement the `PhantomCType` trait.

```rust
struct PhantomCTypeVTable {
    metadata_fn: fn() -> &'static dyn Provider,
}

impl PhantomCType for PhantomCTypeVTable {
    fn metadata(&self) -> &'static dyn Provider {
        (self.metadata_fn)()
    }
}

impl PhantomCTypeVTable {
    fn from_ctype<T: CType>() -> Self {
        Self {
            metadata_fn: T::metadata,
        }
    }
}
```

And the last thing we need to do is to fix our failing line:

```rust
impl<T: CType> CType for NonNullCLayout<T> {
    fn metadata() -> &'static dyn Provider {
        let usage = MetadataTypeUsage::NonNull {
            inner: Box::new(PhantomCTypeVTable::from_ctype::<T>()),
        };
        let provider = Box::new(ProviderWrapper(usage));
        Box::leak(provider)
    }
}
```


## Use in Rust standard library (applicable to embedded system)
There is one place in the Rust standard library (that I know of) were this patter is used for type erasure and even with an interesting property - in type erasure of the [`std::task::Waker`](https://doc.rust-lang.org/beta/std/task/struct.Waker.html).

Not only does this use allow for use without `dyn`, which is beneficial for embedded systems as it allows for better inlining, but also allows us to write executor agnostic code, because executor's implementation details are hidden in the `*data` and `vtable` fields of the `RawWaker` struct, which represents a Fat pointer to the waker.
