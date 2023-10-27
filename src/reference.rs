use dex::{string::DexString, jtype::Type as JType, field::FieldIdItem, method::{MethodIdItem, MethodHandleItem, ProtoIdItem}, Dex};

pub(crate) enum Type {
    String,
    Type,
    Field,
    Method,
    MethodHandle,
    Prototype,
    CallSite,
}

#[derive(Debug)]
pub(crate) enum Reference {
    String(DexString),
    Type(JType),
    Field(FieldIdItem),
    Method(MethodIdItem),
    MethodHandle(MethodHandleItem),
    Prototype(ProtoIdItem),
    CallSite(u16),
}

#[macro_export]
macro_rules! ref_from_dex {
    ($t:expr, $index:expr, $dex:expr) => {
        match $t {
            Type::String => Reference::String($dex.get_string($index.into()).expect(format!("failed to get string at index {}", $index).as_str())),
            Type::Type => Reference::Type($dex.get_type($index.into()).expect(format!("failed to get type at index {}", $index).as_str())),
            Type::Field => Reference::Field($dex.get_field_item($index.into()).expect(format!("failed to get field item at index {}", $index).as_str())),
            Type::Method => Reference::Method($dex.get_method_item($index.into()).expect(format!("failed to get method item at index {}", $index).as_str())),
            Type::MethodHandle => Reference::MethodHandle($dex.get_method_handle_item($index.into()).expect(format!("failed to get method handle at index {}", $index).as_str())),
            Type::Prototype => Reference::Prototype($dex.get_proto_item($index.into()).expect(format!("failed to get prototype at index {}", $index).as_str())),
            Type::CallSite => Reference::CallSite($index as u16),
        }
    };
}


// impl Reference {
//     pub fn from_dex<T>(t: Type, index: u64, dex: &Dex<&[u8]>) -> Self {
        
//     }
// }