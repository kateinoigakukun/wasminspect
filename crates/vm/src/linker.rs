use crate::module::ModuleIndex;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// An address value which points an `Item` in `LinkableCollection`
/// The pointee item must be exists in the collection.
#[derive(PartialEq, Eq, Hash)]
pub struct GlobalAddress<Item>(usize, std::marker::PhantomData<Item>);

impl<Item> Clone for GlobalAddress<Item> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<Item> Copy for GlobalAddress<Item> {}

impl<Item> fmt::Debug for GlobalAddress<Item> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlobalAddress({})", self.0)
    }
}

/// An address value which *may* points an `Item` in `LinkableCollection`
/// or another `LinkableAddress<Item>`.
/// To access the pointee, resolve it by `LinkableCollection`,
/// and get a real address `GlobalAddress`.
/// Note that different `LinkableAddress`es can points the same item.
pub struct LinkableAddress<Item>(
    ModuleIndex,
    pub(crate) usize,
    std::marker::PhantomData<fn() -> Item>,
);

impl<Item> LinkableAddress<Item> {
    pub fn new_unsafe(module: ModuleIndex, index: usize) -> Self {
        Self(module, index, std::marker::PhantomData)
    }

    pub fn module_index(&self) -> ModuleIndex {
        self.0
    }
}

impl<Item> Clone for LinkableAddress<Item> {
    fn clone(&self) -> Self {
        Self::new_unsafe(self.0, self.1)
    }
}

impl<Item> Copy for LinkableAddress<Item> {}

impl<Item> fmt::Debug for LinkableAddress<Item> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}, func_index: {}", self.0, self.1)
    }
}

impl<Item> PartialEq for LinkableAddress<Item> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<Item> Eq for LinkableAddress<Item> {}
impl<Item> Hash for LinkableAddress<Item> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        state.write_usize(self.1);
    }
}

/// A collection of items and address spaces that can be linked to each other.
///
/// ## Concept
///
/// `LinkableCollection` holds a collection of `Item`s and two addres spaces.
/// One of the two types of address, `LinkableAddress`, points to another
///  `LinkableAddress` or a `GlobalAddress`. The other address `GlobalAddress`
/// points to an `Item` held by the collection.
///
/// ```text
/// ┌──────────────── LinkableCollection ──────────────────────┐
/// │                                                          │
/// │ LA = `LinkableAddress`, GA = `GlobalAddress`             │
/// │                                                          │
/// │ ┌─── Module A ───┐  ┌── Module B ──┐  ┌─── Module C ───┐ │
/// │ │                │  │              │  │                │ │
/// │ │  LA 0    LA 1  │  │     LA 2 ◀───┼──┼─ LA 3 ◀─ LA 4  │ │
/// │ │    │      │    │  │       │      │  │    ▲           │ │
/// │ └────┼──────┼────┘  └───────┼──────┘  └────┼───────────┘ │
/// │      │      └───────────────│──────────────┘             │
/// │      │                      │                            │
/// │ ┌────┼─── GlobalAddresses ──┼──────────────────────────┐ │
/// │ │    ▼                      ▼                          │ │
/// │ │  ┌──────────┐       ┌──────────┐                     │ │
/// │ │  │   GA 0   │       │   GA 1   │                     │ │
/// │ │  └──────────┘       └──────────┘                     │ │
/// │ └────┼──────────────────────┼──────────────────────────┘ │
/// │      │                      │                            │
/// │ ┌────┼─────── Items ────────┼──────────────────────────┐ │
/// │ │    ▼                      ▼                          │ │
/// │ │  ┌──────────┐       ┌──────────┐                     │ │
/// │ │  │  Item X  │       │  Item Y  │                     │ │
/// │ │  └──────────┘       └──────────┘                     │ │
/// │ └──────────────────────────────────────────────────────┘ │
/// └──────────────────────────────────────────────────────────┘
/// ```
///
pub(crate) struct LinkableCollection<Item> {
    items: Vec<Item>,
    item_addrs_by_module: HashMap<ModuleIndex, Vec<usize>>,
}

impl<Item> Default for LinkableCollection<Item> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            item_addrs_by_module: HashMap::new(),
        }
    }
}

impl<Item> LinkableCollection<Item> {
    pub(crate) fn resolve(&self, address: LinkableAddress<Item>) -> Option<GlobalAddress<Item>> {
        let raw_address = self.item_addrs_by_module.get(&address.0)?.get(address.1)?;
        Some(GlobalAddress(*raw_address, std::marker::PhantomData))
    }

    pub(crate) fn link(
        &mut self,
        source: GlobalAddress<Item>,
        dist: ModuleIndex,
    ) -> LinkableAddress<Item> {
        let index = self
            .item_addrs_by_module
            .get(&dist)
            .map(|c| c.len())
            .unwrap_or(0);
        self.item_addrs_by_module
            .entry(dist)
            .or_insert_with(Vec::new)
            .push(source.0);
        LinkableAddress::new_unsafe(dist, index)
    }

    pub(crate) fn get_global(&self, address: GlobalAddress<Item>) -> &Item {
        // Never panic because GlobalAddress is always valid
        self.items.get(address.0).unwrap()
    }

    pub(crate) fn get(
        &self,
        address: LinkableAddress<Item>,
    ) -> Option<(&Item, GlobalAddress<Item>)> {
        let addr = self.resolve(address)?;
        Some((self.items.get(addr.0)?, addr))
    }

    pub(crate) fn push_global(&mut self, item: Item) -> GlobalAddress<Item> {
        let index = self.items.len();
        self.items.push(item);
        GlobalAddress(index, std::marker::PhantomData)
    }

    pub(crate) fn push(&mut self, module_index: ModuleIndex, item: Item) -> LinkableAddress<Item> {
        let globa_index = self.items.len();
        self.items.push(item);
        let addrs = self
            .item_addrs_by_module
            .entry(module_index)
            .or_insert_with(Vec::new);
        let index = addrs.len();
        addrs.push(globa_index);
        LinkableAddress::new_unsafe(module_index, index)
    }

    pub(crate) fn items(&self, module_index: ModuleIndex) -> Option<Vec<GlobalAddress<Item>>> {
        let item_addrs = self.item_addrs_by_module.get(&module_index)?;
        Some(
            item_addrs
                .iter()
                .map(|index| GlobalAddress(*index, std::marker::PhantomData))
                .collect(),
        )
    }

    pub(crate) fn is_empty(&self, module_index: ModuleIndex) -> bool {
        self.item_addrs_by_module
            .get(&module_index)
            .map(|v| v.is_empty())
            .unwrap_or(true)
    }
}
