use zbus::{block_on, fdo::ObjectManager};

mod contract {
    #[zbus::interface(
        name = "org.zbus.TraitContract",
        property_snapshot(derive(Clone, PartialEq, Eq)),
        proxy(default_service = "org.zbus.TraitContractTest", visibility = "pub")
    )]
    pub trait TraitContract {
        async fn greet(&self, name: String) -> String;

        fn get_managed_objects_typed(&self) -> String;

        #[zbus(property)]
        fn set_count(&mut self, value: u32);

        #[zbus(property)]
        fn count(&self) -> u32;

        #[zbus(property)]
        fn label(&self) -> zbus::fdo::Result<String>;

        #[zbus(property, object_manager(skip))]
        fn costly_detail(&self) -> String;
    }
}

mod implementation {
    use super::contract::TraitContract;

    pub struct Service {
        count: u32,
    }

    impl Service {
        pub fn new(count: u32) -> Self {
            Self { count }
        }
    }

    impl TraitContract for Service {
        async fn greet(&self, name: String) -> String {
            format!("Hello, {name}!")
        }

        fn get_managed_objects_typed(&self) -> String {
            "contract method".into()
        }

        fn count(&self) -> u32 {
            self.count
        }

        fn set_count(&mut self, value: u32) {
            self.count = value;
        }

        fn label(&self) -> zbus::fdo::Result<String> {
            Ok("primary".into())
        }

        fn costly_detail(&self) -> String {
            "still-on-the-standard-wire".into()
        }
    }
}

struct Unrelated;

#[zbus::interface(name = "org.zbus.Unrelated")]
impl Unrelated {
    #[zbus(property)]
    fn value(&self) -> u32 {
        99
    }
}

#[test]
fn trait_contract_end_to_end() {
    block_on(async {
        #[cfg(feature = "blocking-api")]
        use contract::TraitContractProxyBlocking;
        use contract::{
            TraitContractManagedProperties, TraitContractObjectManagerProxyExt,
            TraitContractProperties, TraitContractProxy, TraitContractServer,
        };
        use implementation::Service;

        const ROOT: &str = "/org/zbus/TraitContractTest";
        const ITEM: &str = "/org/zbus/TraitContractTest/item";

        let service = zbus::connection::Builder::session()
            .unwrap()
            .name("org.zbus.TraitContractTest")
            .unwrap()
            .serve_at(ITEM, TraitContractServer(Service::new(4)))
            .unwrap()
            .serve_at(ITEM, Unrelated)
            .unwrap()
            .serve_at(ROOT, ObjectManager)
            .unwrap()
            .build()
            .await
            .unwrap();
        let client = zbus::Connection::session().await.unwrap();

        let proxy = TraitContractProxy::builder(&client)
            .path(ITEM)
            .unwrap()
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
            .unwrap();
        assert_eq!(proxy.greet("world".into()).await.unwrap(), "Hello, world!");
        assert_eq!(
            proxy.get_managed_objects_typed().await.unwrap(),
            "contract method"
        );
        assert_eq!(proxy.count().await.unwrap(), 4);
        proxy.set_count(8).await.unwrap();
        assert_eq!(proxy.count().await.unwrap(), 8);
        assert_eq!(proxy.label().await.unwrap(), "primary");
        assert_eq!(
            proxy.costly_detail().await.unwrap(),
            "still-on-the-standard-wire"
        );

        let full_reply = client
            .call_method(
                Some("org.zbus.TraitContractTest"),
                ITEM,
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &(TraitContractProperties::INTERFACE_NAME,),
            )
            .await
            .unwrap();
        let full_snapshot: TraitContractProperties = full_reply.body().deserialize().unwrap();
        assert_eq!(full_snapshot.count, 8);
        assert_eq!(full_snapshot.costly_detail, "still-on-the-standard-wire");
        assert_eq!(full_snapshot.clone(), full_snapshot);

        let standard_manager = zbus::fdo::ObjectManagerProxy::builder(&client)
            .destination("org.zbus.TraitContractTest")
            .unwrap()
            .path(ROOT)
            .unwrap()
            .build()
            .await
            .unwrap();
        let standard_objects = standard_manager.get_managed_objects().await.unwrap();
        let standard_properties = standard_objects
            .get(&zbus::zvariant::OwnedObjectPath::try_from(ITEM).unwrap())
            .unwrap()
            .get(TraitContractProperties::INTERFACE_NAME)
            .unwrap();
        assert!(standard_properties.contains_key("CostlyDetail"));

        let manager = TraitContractProxy::builder(&client)
            .path(ROOT)
            .unwrap()
            .build()
            .await
            .unwrap();
        let objects = TraitContractObjectManagerProxyExt::get_managed_objects_typed(&manager)
            .await
            .unwrap();
        let item_path = zbus::zvariant::ObjectPath::try_from(ITEM).unwrap();
        let snapshot = objects
            .get(&item_path)
            .unwrap()
            .trait_contract
            .as_ref()
            .unwrap();
        assert_eq!(snapshot.count, 8);
        assert_eq!(snapshot.label.as_deref(), Some("primary"));
        let _: &TraitContractManagedProperties = snapshot;
        assert_eq!(
            TraitContractProperties::INTERFACE_NAME,
            "org.zbus.TraitContract"
        );
        assert_eq!(
            TraitContractProperties::MUTABLE_PROPERTY_NAMES,
            &["CostlyDetail", "Count", "Label"]
        );

        // Both proxy flavors are part of the generated public contract.
        #[cfg(feature = "blocking-api")]
        assert_ne!(
            std::mem::size_of::<TraitContractProxyBlocking<'static>>(),
            0
        );
        drop(service);
    });
}
