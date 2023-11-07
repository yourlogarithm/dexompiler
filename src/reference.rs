use dex::{string::DexString, jtype::Type as JType, field::FieldIdItem, method::{MethodIdItem, MethodHandleItem, ProtoIdItem}, Dex};

pub enum Type {
    String,
    Type,
    Field,
    Method,
    MethodHandle,
    Prototype,
    CallSite,
}

#[derive(Debug, PartialEq)]
pub enum Item {
    String(DexString),
    Type(JType),
    Field(FieldIdItem),
    Method(MethodIdItem),
    MethodHandle(MethodHandleItem),
    Prototype(ProtoIdItem),
    CallSite(u16),
}

impl Item {
    pub fn from_short_index(t: Type, index: u16, dex: &Dex<impl AsRef<[u8]>>) -> Self {
        match t {
            Type::String => Item::String(dex.get_string(index as u32).expect(format!("failed to get string at index {}", index).as_str())),
            Type::Type => Item::Type(dex.get_type(index as u32).expect(format!("failed to get type at index {}", index).as_str())),
            Type::MethodHandle => Item::MethodHandle(dex.get_method_handle_item(index as u32).expect(format!("failed to get method handle at index {}", index).as_str())),
            Type::CallSite => Item::CallSite(index),
            Type::Field => Item::Field(dex.get_field_item(index as u64).expect(format!("failed to get field item at index {}", index).as_str())),
            Type::Method => Item::Method(dex.get_method_item(index as u64).expect(format!("failed to get method item at index {}", index).as_str())),
            Type::Prototype => Item::Prototype(dex.get_proto_item(index as u64).expect(format!("failed to get prototype at index {}", index).as_str())),
            _ => panic!("Invalid type `t` for `from_short_index`, use `from_long_index`")
        }
    }

    pub fn from_index(t: Type, index: u32, dex: &Dex<impl AsRef<[u8]>>) -> Self {
        match t {
            Type::String => Item::String(dex.get_string(index).expect(format!("failed to get string at index {}", index).as_str())),
            Type::Type => Item::Type(dex.get_type(index).expect(format!("failed to get type at index {}", index).as_str())),
            Type::MethodHandle => Item::MethodHandle(dex.get_method_handle_item(index as u32).expect(format!("failed to get method handle at index {}", index).as_str())),
            _ => panic!("Invalid type `t` for `from_index`, use `from_long_index` or `from_short_index` instead")
        }
    }
}

