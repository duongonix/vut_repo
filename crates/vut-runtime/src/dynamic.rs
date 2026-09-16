use std::{any::Any, sync::Arc};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodTable {
    pub interface_id: u64,
    pub methods: Arc<[usize]>,
}
#[derive(Clone, Debug)]
pub struct InterfaceValue<T> {
    pub data: T,
    pub table: MethodTable,
}
pub struct DynValue {
    type_name: &'static str,
    value: Box<dyn Any>,
}
impl DynValue {
    pub fn new<T: Any>(value: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            value: Box::new(value),
        }
    }
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.value.downcast_ref()
    }
}
