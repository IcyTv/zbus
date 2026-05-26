use quick_xml::de::DeError;
use std::error::Error;

use zbus_xml::{ArgDirection, Node};
use zvariant::Signature;

#[test]
fn serde() -> Result<(), Box<dyn Error>> {
    let example = include_str!("data/sample_object0.xml");
    let node_r = Node::from_reader(example.as_bytes())?;
    let node = Node::try_from(example)?;
    assert_eq!(node, node_r);
    assert_eq!(node.interfaces().len(), 1);
    assert_eq!(node.interfaces()[0].methods().len(), 3);
    assert_eq!(
        node.interfaces()[0].methods()[0].args()[0]
            .direction()
            .unwrap(),
        ArgDirection::In
    );
    assert_eq!(node.nodes().len(), 4);

    let node_str: Node<'_> = example.try_into()?;
    assert_eq!(node_str.interfaces().len(), 1);
    assert_eq!(node_str.nodes().len(), 4);

    let mut writer = Vec::with_capacity(128);
    node.to_writer(&mut writer).unwrap();
    Ok(())
}

#[test]
fn invalid_arg_type() {
    let input = include_str!("data/invalid_arg_type.xml");
    assert!(matches!(
        Node::try_from(input),
        Err(zbus_xml::Error::QuickXml(DeError::Custom(_)))
    ));
}

#[test]
fn multi_complete_arg_type() -> Result<(), Box<dyn Error>> {
    let input = r#"
        <!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
        <node>
            <interface name="org.test.testinterface">
                <method name="testmethod">
                    <arg name="testarg" direction="out" type="tt"/>
                </method>
            </interface>
        </node>
    "#;

    let node = Node::try_from(input)?;
    let arg = &node.interfaces()[0].methods()[0].args()[0];
    let Signature::Structure(fields) = arg.ty().inner() else {
        panic!("expected `tt` to parse as a structure");
    };

    assert_eq!(fields.len(), 2);
    assert_eq!(fields.get(0), Some(&Signature::U64));
    assert_eq!(fields.get(1), Some(&Signature::U64));

    Ok(())
}
