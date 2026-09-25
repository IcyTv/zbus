#[renamed_zbus::interface(
    name = "org.example.RenamedZbus",
    crate = "renamed_zbus",
    property_snapshot(derive(Clone, PartialEq, Eq)),
    proxy(gen_blocking = false)
)]
pub trait Contract {
    fn ping(&self) -> u32;

    #[zbus(property)]
    fn set_value(&mut self, value: u32);

    #[zbus(property)]
    fn value(&self) -> u32;
}

pub struct Service(pub u32);

impl Contract for Service {
    fn ping(&self) -> u32 {
        self.0
    }

    fn set_value(&mut self, value: u32) {
        self.0 = value;
    }

    fn value(&self) -> u32 {
        self.0
    }
}

pub fn server() -> ContractServer<Service> {
    ContractServer(Service(42))
}
