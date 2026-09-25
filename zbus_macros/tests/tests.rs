use futures_util::{
    future::{Either, select},
    stream::StreamExt,
};
use std::future::ready;
use zbus::{block_on, fdo, object_server::SignalEmitter, proxy::CacheProperties};
use zbus_macros::{DBusError, interface, proxy};

pub mod private_trait_public_proxy {
    #[zbus::interface(
        name = "org.example.PrivateTraitPublicProxy",
        proxy(visibility = "pub")
    )]
    #[allow(dead_code)]
    trait Contract {
        fn ping(&self);
    }
}

#[test]
fn trait_interface_typed_helpers_do_not_exceed_contract_visibility() {
    assert_ne!(
        std::mem::size_of::<private_trait_public_proxy::ContractProxy<'static>>(),
        0
    );
}

mod trait_interface {
    use zbus::interface;

    #[interface(
        name = "org.freedesktop.zbus_macros.TraitInterface",
        server_name = "ServiceAdapter",
        property_snapshot_name = "Snapshot",
        managed_property_snapshot_name = "ManagedSnapshot",
        object_manager_name = "ManagedObject",
        property_snapshot(derive(Clone, PartialEq, Eq)),
        proxy(
            async_name = "Client",
            blocking_name = "BlockingClient",
            visibility = "pub(crate)"
        )
    )]
    pub trait Contract {
        fn echo(&self, value: String) -> String;

        #[zbus(property)]
        fn set_count(&mut self, value: u32);

        #[zbus(property)]
        fn count(&self) -> u32;

        #[zbus(property)]
        fn label(&self) -> zbus::fdo::Result<String>;

        #[zbus(property, object_manager(skip))]
        fn expensive(&self) -> String;

        #[zbus(property(emits_changed_signal = "const"))]
        fn version(&self) -> u32;

        #[zbus(property(emits_changed_signal = "false"))]
        fn quiet(&self) -> bool;

        #[cfg_attr(any(), inline)]
        #[zbus(property)]
        fn phantom(&self) -> u32;

        #[zbus(property)]
        fn set_title(&mut self, value: &str);

        #[zbus(property)]
        fn title(&self) -> String;

        #[cfg(any())]
        #[zbus(signal)]
        async fn disabled_signal(emitter: &zbus::object_server::SignalEmitter<'_>);

        #[cfg(any())]
        #[zbus(property)]
        fn set_disabled(&mut self, value: u32);

        #[cfg(any())]
        #[zbus(property)]
        fn disabled(&self) -> u32;
    }

    pub struct Behavior(pub u32);

    impl Contract for Behavior {
        fn echo(&self, value: String) -> String {
            value
        }

        fn count(&self) -> u32 {
            self.0
        }

        fn set_count(&mut self, value: u32) {
            self.0 = value;
        }

        fn label(&self) -> zbus::fdo::Result<String> {
            Ok("test".into())
        }

        fn expensive(&self) -> String {
            "ignored".into()
        }

        fn version(&self) -> u32 {
            1
        }

        fn quiet(&self) -> bool {
            true
        }

        fn phantom(&self) -> u32 {
            13
        }

        fn set_title(&mut self, _value: &str) {}

        fn title(&self) -> String {
            "title".into()
        }
    }
}

#[test]
fn trait_interface_generates_server_proxy_and_typed_properties() {
    use std::collections::HashMap;
    use trait_interface::{ManagedObject, ManagedSnapshot, ServiceAdapter, Snapshot};
    use zbus::{object_server::Interface as _, zvariant};

    let adapter = ServiceAdapter(trait_interface::Behavior(7));
    assert_eq!(adapter.0.0, 7);
    assert_eq!(
        ServiceAdapter::<trait_interface::Behavior>::name(),
        "org.freedesktop.zbus_macros.TraitInterface"
    );
    assert_eq!(
        Snapshot::INTERFACE_NAME,
        "org.freedesktop.zbus_macros.TraitInterface"
    );
    assert_eq!(
        Snapshot::PROPERTY_NAMES,
        &[
            "Count",
            "Expensive",
            "Label",
            "Phantom",
            "Quiet",
            "Title",
            "Version"
        ]
    );
    assert_eq!(
        Snapshot::MUTABLE_PROPERTY_NAMES,
        &["Count", "Expensive", "Label", "Phantom", "Quiet", "Title"]
    );

    let context = zvariant::serialized::Context::new_dbus(zvariant::LE, 0);
    let wire = HashMap::from([
        ("Count", zvariant::Value::new(42u32)),
        (
            "Expensive",
            zvariant::Value::new("decoded in full snapshot"),
        ),
        ("Label", zvariant::Value::new("hello")),
        ("Phantom", zvariant::Value::new(13u32)),
        ("Quiet", zvariant::Value::new(true)),
        ("Title", zvariant::Value::new("title")),
        ("Version", zvariant::Value::new(1u32)),
        ("Unknown", zvariant::Value::new(true)),
    ]);
    let encoded = zvariant::to_bytes(context, &wire).unwrap();
    let snapshot: Snapshot = encoded.deserialize().unwrap().0;
    assert_eq!(snapshot.count, 42);
    assert_eq!(snapshot.expensive, "decoded in full snapshot");
    assert_eq!(snapshot.label.as_deref(), Some("hello"));
    assert_eq!(snapshot.phantom, 13);
    assert!(snapshot.quiet);
    assert_eq!(snapshot.title, "title");
    assert_eq!(snapshot.version, 1);
    assert_eq!(snapshot.clone(), snapshot);

    let encoded_snapshot = zvariant::to_bytes(context, &snapshot).unwrap();
    let round_trip: Snapshot = encoded_snapshot.deserialize().unwrap().0;
    assert_eq!(round_trip, snapshot);

    let encoded = zvariant::to_bytes(context, &wire).unwrap();
    let managed: ManagedSnapshot = encoded.deserialize().unwrap().0;
    assert_eq!(managed.count, 42);
    assert_eq!(managed.version, 1);

    let interfaces = HashMap::from([
        ("org.freedesktop.zbus_macros.TraitInterface", wire),
        ("org.example.Unknown", HashMap::new()),
    ]);
    let encoded = zvariant::to_bytes(context, &interfaces).unwrap();
    let object: ManagedObject = encoded.deserialize().unwrap().0;
    assert_eq!(object.contract.unwrap().count, 42);

    let missing_required = HashMap::from([("Label", zvariant::Value::new("hello"))]);
    let encoded = zvariant::to_bytes(context, &missing_required).unwrap();
    assert!(encoded.deserialize::<Snapshot>().is_err());
}

mod param {
    #[zbus_macros::proxy(
        interface = "org.freedesktop.zbus_macros.ProxyParam",
        default_service = "org.freedesktop.zbus_macros",
        default_path = "/org/freedesktop/zbus_macros/test"
    )]
    trait ProxyParam {
        #[zbus(object = "super::test::Test")]
        fn some_method<T>(&self, test: &T);
    }
}

mod const_attrs {
    pub const INTERFACE: zbus::names::InterfaceName<'static> =
        zbus::names::InterfaceName::from_static_str_checked(
            "org.freedesktop.zbus_macros.ConstAttrs",
        );
    pub const SERVICE: zbus::names::BusName<'static> =
        zbus::names::BusName::from_static_str_checked("org.freedesktop.zbus_macros.ConstAttrs");
    pub const PATH: zbus::zvariant::ObjectPath<'static> =
        zbus::zvariant::ObjectPath::from_static_str_checked(
            "/org/freedesktop/zbus_macros/ConstAttrs",
        );

    #[zbus_macros::proxy(interface = INTERFACE, default_service = SERVICE, default_path = PATH)]
    pub trait ConstAttrs {
        fn ping(&self) -> zbus::Result<()>;

        #[zbus(signal)]
        fn pong(&self) -> zbus::Result<()>;
    }

    pub struct ConstAttrsInterface;

    #[zbus_macros::interface(name = INTERFACE)]
    impl ConstAttrsInterface {
        fn ping(&self) {}
    }
}

#[test]
fn test_const_attrs() {
    use zbus::{object_server::Interface as _, proxy::Defaults as _};

    assert_eq!(
        const_attrs::ConstAttrsProxy::INTERFACE.as_ref(),
        Some(&const_attrs::INTERFACE),
    );
    assert_eq!(
        const_attrs::ConstAttrsProxy::DESTINATION.as_ref(),
        Some(&const_attrs::SERVICE),
    );
    assert_eq!(
        const_attrs::ConstAttrsProxy::PATH.as_ref(),
        Some(&const_attrs::PATH),
    );
    assert_eq!(
        const_attrs::ConstAttrsInterface::name(),
        const_attrs::INTERFACE,
    );

    let mut xml = String::new();
    const_attrs::ConstAttrsInterface.introspect_to_writer(&mut xml, 0);
    assert_eq!(
        xml,
        r#"<interface name="org.freedesktop.zbus_macros.ConstAttrs">
  <method name="Ping">
  </method>
</interface>
"#
    );
}

mod test {
    use zbus::{
        fdo,
        zvariant::{OwnedStructure, Structure},
    };

    #[zbus_macros::proxy(
        assume_defaults = false,
        interface = "org.freedesktop.zbus_macros.Test",
        default_service = "org.freedesktop.zbus_macros"
    )]
    pub(super) trait Test {
        /// comment for a_test()
        fn a_test(&self, val: &str) -> zbus::Result<u32>;

        /// The generated proxies implement both `zvariant::Type` and `serde::ser::Serialize`
        /// which is useful to pass in a proxy as a param. It serializes it as an `ObjectPath`.
        fn some_method<T>(&self, object_path: &T) -> zbus::Result<()>;

        /// A call accepting an argument that only implements DynamicType and Serialize.
        fn test_dyn_type(&self, arg: Structure<'_>, arg2: u32) -> zbus::Result<()>;

        /// A call returning an type that only implements DynamicDeserialize
        fn test_dyn_ret(&self) -> zbus::Result<OwnedStructure>;

        #[zbus(name = "CheckRENAMING")]
        fn check_renaming(&self) -> zbus::Result<Vec<u8>>;

        #[zbus(property)]
        fn property(&self) -> fdo::Result<Vec<String>>;

        #[zbus(property(emits_changed_signal = "const"))]
        fn a_const_property(&self) -> fdo::Result<Vec<String>>;

        #[zbus(property(emits_changed_signal = "false"))]
        fn a_live_property(&self) -> fdo::Result<Vec<String>>;

        #[zbus(property)]
        fn set_property(&self, val: u16) -> fdo::Result<()>;

        #[zbus(signal)]
        fn a_signal<T>(&self, arg: u8, other: T) -> fdo::Result<()>
        where
            T: AsRef<str>;
    }
}

#[test]
fn test_proxy() {
    block_on(async move {
        let connection = zbus::Connection::session().await.unwrap();
        let proxy = test::TestProxy::builder(&connection)
            .path("/org/freedesktop/zbus_macros/test")
            .unwrap()
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .unwrap();
        fdo::DBusProxy::builder(&connection)
            .build()
            .await
            .unwrap()
            .request_name(
                "org.freedesktop.zbus_macros".try_into().unwrap(),
                fdo::RequestNameFlags::DoNotQueue.into(),
            )
            .await
            .unwrap();
        let mut stream = proxy.receive_a_signal().await.unwrap();

        let left_future = async move {
            // These calls will never happen so just testing the build mostly.
            let signal = stream.next().await.unwrap();
            let args = signal.args::<&str>().unwrap();
            assert_eq!(*args.arg(), 0u8);
            assert_eq!(*args.other(), "whatever");
        };
        futures_util::pin_mut!(left_future);
        let right_future = async {
            ready(()).await;
        };
        futures_util::pin_mut!(right_future);

        if let Either::Left((_, _)) = select(left_future, right_future).await {
            panic!("Shouldn't be receiving our dummy signal: `ASignal`");
        }
    });
}

#[ignore]
#[test]
fn test_derive_error() {
    #[allow(unused)]
    #[derive(Debug, DBusError)]
    #[zbus(prefix = "org.freedesktop.zbus")]
    enum Test {
        #[zbus(error)]
        ZBus(zbus::Error),
        SomeExcuse,
        #[zbus(name = "I.Am.Sorry.Dave")]
        IAmSorryDave(String),
        LetItBe {
            desc: String,
        },
    }
}

#[test]
fn test_interface() {
    use serde::{Deserialize, Serialize};
    use zbus::{
        object_server::Interface,
        zvariant::{Type, Value},
    };

    // Test write-only property
    struct TestWriteOnlyProperty;

    #[interface(proxy)]
    impl TestWriteOnlyProperty {
        #[zbus(property)]
        fn set_my_property(&self, _val: u32) {}
    }

    let mut writer = String::new();
    TestWriteOnlyProperty.introspect_to_writer(&mut writer, 0);
    assert_eq!(
        writer,
        r#"<interface name="org.freedesktop.TestWriteOnlyProperty">
  <property name="MyProperty" type="u" access="write">
    <annotation name="org.freedesktop.DBus.Property.EmitsChangedSignal" value="false"/>
  </property>
</interface>
"#
    );

    struct Test<T> {
        something: String,
        generic: T,
    }

    #[derive(Serialize, Deserialize, Type, Value)]
    struct MyCustomPropertyType(u32);

    #[interface(name = "org.freedesktop.zbus.Test", spawn = false)]
    impl<T: 'static> Test<T>
    where
        T: serde::ser::Serialize + zbus::zvariant::Type + Send + Sync,
    {
        /// Testing `no_arg` documentation is reflected in XML.
        fn no_arg(&self) {
            unimplemented!()
        }

        // Also tests that mut argument bindings work for regular methods
        #[allow(unused_assignments)]
        fn str_u32(&self, mut val: &str) -> zbus::fdo::Result<u32> {
            let res = val
                .parse()
                .map_err(|e| zbus::fdo::Error::Failed(format!("Invalid val: {e}")));
            val = "test mut";
            res
        }

        // TODO: naming output arguments after "RFC: Structural Records #2584"
        fn many_output(&self) -> zbus::fdo::Result<(&T, String)> {
            Ok((&self.generic, self.something.clone()))
        }

        fn pair_output(&self) -> zbus::fdo::Result<((u32, String),)> {
            unimplemented!()
        }

        #[zbus(property)]
        fn my_custom_property(&self) -> MyCustomPropertyType {
            unimplemented!()
        }

        // Also tests that mut argument bindings work for properties
        #[zbus(property)]
        fn set_my_custom_property(&self, mut _value: MyCustomPropertyType) {
            _value = MyCustomPropertyType(42);
        }

        // Test that the emits_changed_signal property results in the correct annotation
        #[zbus(property(emits_changed_signal = "false"))]
        fn my_custom_property_emits_false(&self) -> MyCustomPropertyType {
            unimplemented!()
        }

        #[zbus(property(emits_changed_signal = "invalidates"))]
        fn my_custom_property_emits_invalidates(&self) -> MyCustomPropertyType {
            unimplemented!()
        }

        #[zbus(property(emits_changed_signal = "const"))]
        fn my_custom_property_emits_const(&self) -> MyCustomPropertyType {
            unimplemented!()
        }

        #[zbus(name = "CheckVEC")]
        fn check_vec(&self) -> Vec<u8> {
            unimplemented!()
        }

        /// Testing my_prop documentation is reflected in XML.
        ///
        /// And that too.
        #[zbus(property)]
        fn my_prop(&self) -> u16 {
            unimplemented!()
        }

        #[zbus(property)]
        fn set_my_prop(&mut self, _val: u16) {
            unimplemented!()
        }

        /// Emit a signal.
        #[zbus(signal)]
        async fn signal(emitter: &SignalEmitter<'_>, arg: u8, other: &str) -> zbus::Result<()>;
    }

    const EXPECTED_XML: &str = r#"<interface name="org.freedesktop.zbus.Test">
  <!--
   Testing `no_arg` documentation is reflected in XML.
   -->
  <method name="NoArg">
  </method>
  <method name="StrU32">
    <arg name="val" type="s" direction="in"/>
    <arg type="u" direction="out"/>
  </method>
  <method name="ManyOutput">
    <arg type="u" direction="out"/>
    <arg type="s" direction="out"/>
  </method>
  <method name="PairOutput">
    <arg type="(us)" direction="out"/>
  </method>
  <method name="CheckVEC">
    <arg type="ay" direction="out"/>
  </method>
  <!--
   Emit a signal.
   -->
  <signal name="Signal">
    <arg name="arg" type="y"/>
    <arg name="other" type="s"/>
  </signal>
  <property name="MyCustomProperty" type="u" access="readwrite"/>
  <property name="MyCustomPropertyEmitsConst" type="u" access="read">
    <annotation name="org.freedesktop.DBus.Property.EmitsChangedSignal" value="const"/>
  </property>
  <property name="MyCustomPropertyEmitsFalse" type="u" access="read">
    <annotation name="org.freedesktop.DBus.Property.EmitsChangedSignal" value="false"/>
  </property>
  <property name="MyCustomPropertyEmitsInvalidates" type="u" access="read">
    <annotation name="org.freedesktop.DBus.Property.EmitsChangedSignal" value="invalidates"/>
  </property>
  <!--
   Testing my_prop documentation is reflected in XML.

   And that too.
   -->
  <property name="MyProp" type="q" access="readwrite"/>
</interface>
"#;
    let t = Test {
        something: String::from("somewhere"),
        generic: 42u32,
    };
    let mut xml = String::new();
    t.introspect_to_writer(&mut xml, 0);
    assert_eq!(xml, EXPECTED_XML);

    assert_eq!(Test::<u32>::name(), "org.freedesktop.zbus.Test");

    if false {
        block_on(async {
            // check compilation
            let c = zbus::Connection::session().await.unwrap();
            let s = c.object_server();
            let m = zbus::message::Message::method_call("/", "StrU32")
                .unwrap()
                .build(&(42,))
                .unwrap();
            let _ = t.call(s, &c, &m, "StrU32".try_into().unwrap());
            let ctxt = SignalEmitter::new(&c, "/does/not/matter").unwrap();
            ctxt.signal(23, "ergo sum").await.unwrap();
        });
    }
}

// Test that the `crate` attribute works for custom crate paths.
mod crate_attr_test {
    #[zbus_macros::proxy(
        interface = "org.freedesktop.zbus_macros.CrateAttrTest",
        default_service = "org.freedesktop.zbus_macros",
        default_path = "/org/freedesktop/zbus_macros/crate_attr_test",
        crate = "zbus"
    )]
    trait CrateAttrTest {
        fn test_method(&self) -> zbus::Result<String>;
    }
}

#[test]
fn test_interface_with_crate_attr() {
    use zbus::object_server::Interface;

    struct CrateAttrInterface;

    #[interface(name = "org.freedesktop.zbus.CrateAttrTest", crate = "zbus")]
    impl CrateAttrInterface {
        fn test_method(&self) -> String {
            "test".to_string()
        }
    }

    assert_eq!(
        CrateAttrInterface::name(),
        "org.freedesktop.zbus.CrateAttrTest"
    );
}

#[test]
fn derive_error_with_crate_attr() {
    #[allow(unused)]
    #[derive(Debug, DBusError)]
    #[zbus(prefix = "org.freedesktop.zbus.test", crate = "zbus")]
    enum CrateAttrError {
        TestError,
    }
}

mod signal_from_message {
    use super::*;
    use zbus::message::Message;

    #[proxy(
        interface = "org.freedesktop.zbus_macros.Test",
        default_service = "org.freedesktop.zbus_macros",
        default_path = "/org/freedesktop/zbus_macros/test"
    )]
    trait Test {
        #[zbus(signal)]
        fn signal_u8(&self, arg: u8) -> fdo::Result<()>;

        #[zbus(signal)]
        fn signal_string(&self, arg: String) -> fdo::Result<()>;
    }

    #[test]
    fn signal_u8() {
        let message = Message::signal(
            "/org/freedesktop/zbus_macros/test",
            "org.freedesktop.zbus_macros.Test",
            "SignalU8",
        )
        .expect("Failed to create signal message builder")
        .build(&(1u8,))
        .expect("Failed to build signal message");

        assert!(
            SignalU8::from_message(message.clone()).is_some(),
            "Message is a SignalU8"
        );
        assert!(
            SignalString::from_message(message).is_none(),
            "Message is not a SignalString"
        );
    }

    #[test]
    fn signal_string() {
        let message = Message::signal(
            "/org/freedesktop/zbus_macros/test",
            "org.freedesktop.zbus_macros.Test",
            "SignalString",
        )
        .expect("Failed to create signal message builder")
        .build(&(String::from("test"),))
        .expect("Failed to build signal message");

        assert!(
            SignalString::from_message(message.clone()).is_some(),
            "Message is a SignalString"
        );
        assert!(
            SignalU8::from_message(message).is_none(),
            "Message is not a SignalU8"
        );
    }

    #[test]
    fn wrong_data() {
        let message = Message::signal(
            "/org/freedesktop/zbus_macros/test",
            "org.freedesktop.zbus_macros.Test",
            "SignalU8",
        )
        .expect("Failed to create signal message builder")
        .build(&(String::from("test"),))
        .expect("Failed to build signal message");

        let signal = SignalU8::from_message(message).expect("Message is a SignalU8");
        signal
            .args()
            .expect_err("Message does not have correct data");
    }
}

#[test]
fn test_proxy_object_list() {
    #[derive(Clone)]
    struct ObjectList {
        paths: [zbus::zvariant::ObjectPath<'static>; 2],
    }

    #[zbus_macros::interface(
        name = "org.freedesktop.zbus_macros.ObjectList",
        proxy(default_service = "org.freedesktop.zbus_macros")
    )]
    impl ObjectList {
        #[zbus(proxy(object = "ObjectList", object_vec))]
        async fn get_test_objects(&self) -> Vec<zbus::zvariant::ObjectPath<'static>> {
            self.paths.to_vec()
        }

        #[zbus(property, proxy(object = "ObjectList", object_vec))]
        fn objects(&self) -> Vec<zbus::zvariant::ObjectPath<'static>> {
            self.paths.to_vec()
        }
    }

    static OBJECT_LIST: ObjectList = ObjectList {
        paths: [
            zbus::zvariant::ObjectPath::from_static_str_unchecked(
                "/org/freedesktop/zbus_macros/object_list/0",
            ),
            zbus::zvariant::ObjectPath::from_static_str_unchecked(
                "/org/freedesktop/zbus_macros/object_list/1",
            ),
        ],
    };

    fn check_return(list: Vec<ObjectListProxyBlocking<'_>>) {
        for (correct, returned) in OBJECT_LIST.paths.iter().zip(list.into_iter()) {
            assert!(returned.inner().path() == correct);
        }
    }

    let connection = zbus::blocking::connection::Builder::session()
        .unwrap()
        .serve_at(OBJECT_LIST.paths[1].as_ref(), OBJECT_LIST.clone())
        .unwrap()
        .build()
        .unwrap();
    let destination = connection.unique_name().unwrap().clone();

    let proxy = ObjectListProxyBlocking::builder(&connection)
        .path(OBJECT_LIST.paths[1].as_ref())
        .unwrap()
        .destination(&destination)
        .unwrap()
        .build()
        .unwrap();

    check_return(proxy.get_test_objects().unwrap());
    check_return(proxy.objects().unwrap());
}
