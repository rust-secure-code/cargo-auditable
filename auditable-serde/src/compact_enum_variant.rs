//! A new enum representation for serde built on generics. Using it requires some extra attributes and impls.
//!
//! This new representation of enum variants makes it much simpler to use much
//! more compact representation of simple string whenever possible e.g. when 
//! all the fields of the variant can be inferred from the implementation of
//! the `Defualt` trait.
//!
//! It is built on the `VariantRepr` type. It is not recommended to use this
//! type directly. Instead for a selected type that appears within a "newtype"
//! variant of an enum certain traits should be implemented.
//!
//! The traits one should implement before using this module within serde's
//! `with` attribute:
//! - `IsEnumVariant<&str, ENUM>` for `VARIANT`
//! - `Into<VariantRepr<&'static str, ENUM, VARIANT>>` for `VARIANT`
//! - `TryFrom<VariantRepr<&'static str, ENUM, VARIANT>>` for `VARIANT`
//!
//! Once those are implemented and the module in which this struct resides is
//! used in serde's attribute as follows:
//! ```rust,ignore
//! #[
//! // ...    
//! ]
//! #[serde(untagged)]
//! pub enum Source {
//! // ...
//! #[serde(with = "compact_enum_variant")]
//! #[cfg_attr(
//!     feature = "schema",
//!     schemars(schema_with = "compact_enum_variant::schema::<Source, GitSource>",)
//! )]
//! Git(GitSource), // Source is ENUM and GitSource is VARIANT
//! // ...
//! }
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, fmt::Display, marker::PhantomData};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(bound = "S: schemars::JsonSchema"))]
pub struct EnumVariant<S: Into<String>, ENUM>(S, PhantomData<fn() -> ENUM>);

pub trait IsEnumVariant<S: Into<String>, ENUM> {
    fn variant() -> EnumVariant<S, ENUM>;
}

impl<S: Into<String>, E> EnumVariant<S, E> {
    pub fn new(variant_tag: S) -> Self {
        Self(variant_tag, PhantomData)
    }
}

impl<S: Into<String>, E> Into<String> for EnumVariant<S, E> {
    fn into(self) -> String {
        self.0.into()
    }
}
impl<E> From<&'static str> for EnumVariant<&'static str, E> {
    fn from(value: &'static str) -> Self {
        EnumVariant::new(value)
    }
}
impl<E> From<&str> for EnumVariant<String, E> {
    fn from(value: &str) -> Self {
        EnumVariant::new(value.to_owned())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[serde(
    untagged,
    bound(
        serialize = "INNER: Serialize, S: 'static + Serialize",
        deserialize = "INNER: Deserialize<'de>, S: Deserialize<'de>"
    )
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum VariantRepr<S: Into<String>, ENUM, INNER> {
    // Short stringly-typed representation of enum variant which assumes all fields
    // are set to their defualts.
    Kind(EnumVariant<S, ENUM>),
    // Longer representation that describes changes to default contents.
    Struct {
        kind: EnumVariant<S, ENUM>,
        #[serde(flatten)]
        strct: INNER,
    },
}

pub fn serialize<S, ENUM, VARIANT>(inner: &VARIANT, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    VARIANT: ToOwned + IsEnumVariant<&'static str, ENUM> + Serialize,
    <VARIANT as ToOwned>::Owned: Into<VariantRepr<&'static str, ENUM, VARIANT>>,
{
    let compact: VariantRepr<&'static str, ENUM, VARIANT> = inner.to_owned().into();

    Serialize::serialize(&compact, serializer)
}

pub fn deserialize<'de, 's, D, ENUM, VARIANT>(deserializer: D) -> Result<VARIANT, D::Error>
where
    D: Deserializer<'de>,
    VARIANT: IsEnumVariant<&'s str, ENUM>
        + TryFrom<VariantRepr<&'s str, ENUM, VARIANT>>
        + Deserialize<'de>,
    <VARIANT as TryFrom<VariantRepr<&'s str, ENUM, VARIANT>>>::Error: Display,
    'de: 's,
{
    let compact: VariantRepr<&'s str, ENUM, VARIANT> = Deserialize::deserialize(deserializer)?;
    let variant = VARIANT::try_from(compact).map_err(|e| serde::de::Error::custom(e))?;

    Ok(variant)
}


/// Enriches the schema generated for `VariantRepr` wtih const values adequate
/// to the selected variant of an enum.
#[cfg(feature = "schema")]
pub fn schema<
    'a,
    ENUM: schemars::JsonSchema,
    VARIANT: Into<VariantRepr<&'static str, ENUM, VARIANT>>
        + IsEnumVariant<&'a str, ENUM>
        + schemars::JsonSchema,
>(
    gen: &mut schemars::gen::SchemaGenerator,
) -> schemars::schema::Schema {
    use schemars::JsonSchema;

    let mut schema =
        <VariantRepr<&'static str, ENUM, VARIANT> as JsonSchema>::json_schema(gen).into_object();

    (&mut schema
        .subschemas)
        .as_mut()
        .and_then(|subschemas| subschemas.any_of.as_mut())
        .and_then(|subschemas| {
            let new_subschemas = subschemas.into_iter().map(|schema| {
                let mut schema = schema.clone().into_object();
                let typ = &schema
                    .instance_type
                    .as_ref()
                    .and_then(|instance_type| match instance_type {
                        schemars::schema::SingleOrVec::Single(typ) => Some(**typ),
                        schemars::schema::SingleOrVec::Vec(_) => None,
                    })
                    .unwrap();
                match typ {
                    schemars::schema::InstanceType::Object => {
                        let object_schema = schema.object();
                        let kind_property = object_schema.properties.get_mut("kind").unwrap();
                        let mut kind_property_object =  kind_property.clone().into_object();
                        kind_property_object.const_value = Some(serde_json::Value::String(VARIANT::variant().into()));
                        *kind_property = schemars::schema::Schema::Object(kind_property_object);
                        
                        schemars::schema::Schema::Object(schema)
                    },
                    schemars::schema::InstanceType::String => {
                        schema.const_value = Some(serde_json::Value::String(VARIANT::variant().into()));
                        schema.string = None;
                        
                        schemars::schema::Schema::Object(schema)                        
                    },
                    _ => panic!("the schema using compact enum variant representation should allow only string or object instances"),
                }
            }).collect();
            *subschemas = new_subschemas;
            Some(subschemas)
        });
        
    schemars::schema::Schema::Object(schema)
}
