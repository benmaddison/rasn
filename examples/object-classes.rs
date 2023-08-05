use std::{fmt, fs::File, io::Read};

use rasn::{
    de, der,
    types::{
        fields::{Field, Fields},
        BitString, Constraints, Constructed, Integer, OctetString, SequenceOf, Utf8String,
    },
    AsnType, Decode, Decoder, Encode, Tag,
};

// SEQUENCE Container
#[derive(Debug, Clone, PartialEq, Eq)]
struct Container {
    version: Integer,
    // it would be nice for this to be SequenceOf<Foo<Box<dyn FooTypeSet>>>,
    // but then we have to specify the type of FooType::Foo :-(
    // ideas?
    foos: SequenceOf<FooEnum>,
}

impl AsnType for Container {
    const TAG: Tag = Tag::SEQUENCE;
}

impl Constructed for Container {
    const FIELDS: rasn::types::fields::Fields = Fields::from_static(&[
        Field::new_required(Integer::TAG, Integer::TAG_TREE),
        Field::new_required(SequenceOf::<FooEnum>::TAG, SequenceOf::<FooEnum>::TAG_TREE),
    ]);
}

impl Decode for Container {
    fn decode_with_tag_and_constraints<D: Decoder>(
        decoder: &mut D,
        tag: Tag,
        _constraints: Constraints,
    ) -> Result<Self, D::Error> {
        decoder.decode_sequence(tag, |decoder| {
            let version = Integer::decode(decoder)?;
            let foos = SequenceOf::decode(decoder)?;
            Ok(Self { version, foos })
        })
    }
}

impl Encode for Container {
    fn encode_with_tag_and_constraints<E: rasn::Encoder>(
        &self,
        encoder: &mut E,
        tag: Tag,
        _constraints: Constraints,
    ) -> Result<(), E::Error> {
        encoder.encode_sequence::<Self, _>(tag, |encoder| {
            self.version.encode(encoder)?;
            self.foos.encode(encoder)?;
            Ok(())
        })?;
        Ok(())
    }
}

// Enumeration of for<T: FooTypeSet> Foo<T>
#[derive(Debug, Clone, PartialEq, Eq)]
enum FooEnum {
    Bar(Foo<FtBar>),
    Baz(Foo<FtBaz>),
}

impl AsnType for FooEnum {
    const TAG: Tag = Tag::SEQUENCE;
}

impl Constructed for FooEnum {
    const FIELDS: Fields = Fields::from_static(&[
        Field::new_required(Utf8String::TAG, Utf8String::TAG_TREE),
        Field::new_required(OctetString::TAG, OctetString::TAG_TREE),
        // Hmmm... this is dependent on the variant of Self
        // Not even quite sure how it is used, since omitting it doesn't seem to break anything!
        // Field::new_required(T::Foo::TAG, T::Foo::TAG_TREE),
    ]);
}

// There is rather a lot of duplication between this and the `impl Decode for Foo<T>`
// Perhaps we only ever want to decode via the enum?
// we could still do:
// let foo_bar = if let FooEnum::Bar(foo) = foo_enum { Ok(foo) } else { Err("wrong foo!")}?;
impl Decode for FooEnum {
    fn decode_with_tag_and_constraints<D: Decoder>(
        decoder: &mut D,
        tag: Tag,
        _constraints: Constraints,
    ) -> Result<Self, D::Error> {
        decoder.decode_sequence(tag, |decoder| {
            let name = Utf8String::decode(decoder)?;
            let foo_type = OctetString::decode(decoder)?;
            match foo_type {
                FtBar::ID => {
                    let data = <FtBar as FooType>::Foo::decode(decoder)?;
                    Ok(Self::Bar(Foo { name, data }))
                }
                FtBaz::ID => {
                    let data = <FtBaz as FooType>::Foo::decode(decoder)?;
                    Ok(Self::Baz(Foo { name, data }))
                }
                _ => Err(de::Error::custom("invalid fooType")),
            }
        })
    }
}

impl Encode for FooEnum {
    fn encode_with_tag_and_constraints<E: rasn::Encoder>(
        &self,
        encoder: &mut E,
        tag: Tag,
        constraints: Constraints,
    ) -> Result<(), E::Error> {
        match self {
            Self::Bar(foo) => foo.encode_with_tag_and_constraints(encoder, tag, constraints),
            Self::Baz(foo) => foo.encode_with_tag_and_constraints(encoder, tag, constraints),
        }
    }
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

impl<T: FooTypeSet> AsnType for Foo<T> {
    const TAG: Tag = Tag::SEQUENCE;
}

impl<T: FooTypeSet> Constructed for Foo<T> {
    const FIELDS: Fields = Fields::from_static(&[
        Field::new_required(Utf8String::TAG, Utf8String::TAG_TREE),
        Field::new_required(OctetString::TAG, OctetString::TAG_TREE),
        Field::new_required(T::Foo::TAG, T::Foo::TAG_TREE),
    ]);
}

impl<T: FooTypeSet> Decode for Foo<T> {
    fn decode_with_tag_and_constraints<D: Decoder>(
        decoder: &mut D,
        tag: Tag,
        constraints: Constraints,
    ) -> Result<Self, D::Error> {
        decoder.decode_sequence(tag, |decoder| {
            let name = Utf8String::decode(decoder)?;
            let foo_type = OctetString::decode(decoder)?;
            let data = T::Foo::decode(decoder)?;
            if foo_type == Self::foo_type() {
                Ok(Self { name, data })
            } else {
                Err(de::Error::custom("invalid fooType"))
            }
        })
    }
}

impl<T: FooTypeSet> Encode for Foo<T> {
    fn encode_with_tag_and_constraints<E: rasn::Encoder>(
        &self,
        encoder: &mut E,
        tag: Tag,
        _constraints: Constraints,
    ) -> Result<(), E::Error> {
        encoder.encode_sequence::<Self, _>(tag, |encoder| {
            self.name.encode(encoder)?;
            Self::foo_type().encode(encoder)?;
            self.data.encode(encoder)?;
            Ok(())
        })?;
        Ok(())
    }
}

// information object class FOO-TYPE
trait FooType {
    const ID: OctetString;
    const DESCR: &'static str;
    type Foo: fmt::Debug + Clone + Eq + AsnType + Decode + Encode;
}

// information object set FooTypeSet
//
// TODO: `UNIQUE` is not enforced anywhere. Perhaps we can lean on the coherence checks somehow?
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
    let expected = Container {
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

    let decoded = der::decode::<Container>(&encoded)?;
    dbg!(&decoded);
    assert_eq!(decoded, expected);
    println!("decoded Container value matches expected data");

    let re_encoded = der::encode(&decoded)?;
    assert_eq!(re_encoded, encoded);
    println!("re-encoded Container value matches original data");

    Ok(())
}
