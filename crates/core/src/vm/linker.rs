use super::module::ModuleIndex;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlobalAddress<T>(usize, std::marker::PhantomData<T>);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkableAddress<T>(ModuleIndex, usize, std::marker::PhantomData<T>);
impl<T> LinkableAddress<T> {
    pub fn new_unsafe(module: ModuleIndex, index: usize) -> Self {
        Self(module, index, std::marker::PhantomData)
    }
}

pub struct LinkableCollection<T> {
    items: Vec<T>,
    item_addrs_by_module: HashMap<ModuleIndex, Vec<usize>>,
}

impl<T> LinkableCollection<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            item_addrs_by_module: HashMap::new(),
        }
    }

    pub fn resolve(&self, address: LinkableAddress<T>) -> Option<GlobalAddress<T>> {
        let raw_address = self.item_addrs_by_module.get(&address.0)?.get(address.1)?;
        Some(GlobalAddress(*raw_address, std::marker::PhantomData))
    }

    pub fn link(&mut self, source: GlobalAddress<T>, dist: ModuleIndex) -> LinkableAddress<T> {
        let index = self
            .item_addrs_by_module
            .get(&dist)
            .map(|c| c.len())
            .unwrap_or(0);
        self.item_addrs_by_module
            .entry(dist)
            .or_insert(Vec::new())
            .push(source.0);
        LinkableAddress::new_unsafe(dist, index)
    }

    pub fn get_global(&self, address: GlobalAddress<T>) -> Option<&T> {
        self.items.get(address.0)
    }

    pub fn get(&self, address: LinkableAddress<T>) -> Option<&T> {
        self.items.get(self.resolve(address)?.0)
    }

    pub fn push_global(&mut self, item: T) -> GlobalAddress<T> {
        let index = self.items.len();
        self.items.push(item);
        GlobalAddress(index, std::marker::PhantomData)
    }

    pub fn push(&mut self, module_index: ModuleIndex, item: T) -> LinkableAddress<T> {
        let globa_index = self.items.len();
        self.items.push(item);
        let addrs = self
            .item_addrs_by_module
            .entry(module_index)
            .or_insert(Vec::new());
        let index = addrs.len();
        addrs.push(globa_index);
        LinkableAddress::new_unsafe(module_index, index)
    }

    pub fn remove_module(&mut self, index: &ModuleIndex) {
        // TODO: GC unlinked items
        self.item_addrs_by_module.remove(index);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
