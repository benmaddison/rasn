use std::{fmt, fs::File, io::Read};

use rasn::types::{BitString, Integer, OctetString, SequenceOf, Utf8String};

// SEQUENCE Container
#[derive(Debug, Clone, PartialEq, Eq)]
struct Container {
    version: Integer,
    // it would be nice for this to be SequenceOf<Foo<Box<dyn FooTypeSet>>>,
    // but then we have to specify the type of FooType::Foo :-(
    // ideas?
    foos: SequenceOf<FooEnum>,
}

// Enumeration of for<T: FooTypeSet> Foo<T>
#[derive(Debug, Clone, PartialEq, Eq)]
enum FooEnum {
    Bar(Foo<FtBar>),
    Baz(Foo<FtBaz>),
}

// SEQUENCE Foo
#[derive(Debug, Clone, PartialEq, Eq)]
struct Foo<T>
where
    // T must be an instance of FOO-TYPE, contained in FooTypeSet
    T: FooTypeSet,
{
    name: Utf8String,
    data: <T as FooType>::Foo,
}

impl<T: FooTypeSet> Foo<T> {
    // implement the fooType field as a const fn as it's value is constrained
    // by <T as Foo>::ID
    const fn foo_type() -> OctetString {
        <T as FooType>::ID
    }
}

// information object class FOO-TYPE
trait FooType {
    const ID: OctetString;
    const DESCR: &'static str;
    type Foo: fmt::Debug + Clone + Eq;
}

// information object set FooTypeSet
trait FooTypeSet
where
    // only instances of FOO-TYPE allowed
    Self: FooType,
{
}

// information object instance ft-Bar
#[derive(Debug, Clone, PartialEq, Eq)]
enum FtBar {}
impl FooType for FtBar {
    const ID: OctetString = OctetString::from_static(&[0x01]);
    const DESCR: &'static str = "Bar";
    type Foo = Integer;
}
// inclusion of ft-Bar in FooTypeSet
impl FooTypeSet for FtBar {}

// information object instance ft-Baz
#[derive(Debug, Clone, PartialEq, Eq)]
enum FtBaz {}
impl FooType for FtBaz {
    const ID: OctetString = OctetString::from_static(&[0x02]);
    const DESCR: &'static str = "Baz";
    type Foo = BitString;
}
// inclusion of ft-Baz in FooTypeSet
impl FooTypeSet for FtBaz {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = {
        let mut buf = Vec::new();
        File::open("examples/object-classes-sample.der")?.read_to_end(&mut buf)?;
        buf
    };
    let decoded = Container {
        version: 0.into(),
        foos: vec![
            FooEnum::Bar(Foo {
                name: "bar1".to_string(),
                data: 1.into(),
            }),
            FooEnum::Baz(Foo {
                name: "baz".to_string(),
                data: [false, true, false, true].iter().collect(),
            }),
            FooEnum::Bar(Foo {
                name: "bar2".to_string(),
                data: 2.into(),
            }),
        ],
    };
    todo!("try to decode `encoded` and check for equality with `decoded`")
}
