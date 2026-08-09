#![allow(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

use mlpl_extension_sdk::{
    EncodedValue, HandleError, HandleRegistry, NativeHandle, Value, copy_foreign_value,
};

#[test]
fn handles_roundtrip_without_raw_pointers() {
    let handle = NativeHandle::from_parts(7, 11, 3, 5);
    let encoded = EncodedValue::new(Value::Handle(handle));
    assert_eq!(
        unsafe { copy_foreign_value(encoded.as_raw()) },
        Ok(Value::Handle(handle))
    );
}

#[test]
fn registry_rejects_stale_wrong_type_and_cross_extension_handles() {
    let mut registry = HandleRegistry::with_limits(7, 2, u32::MAX);
    let first = registry.insert(11, String::from("viewer")).unwrap();
    assert_eq!(
        registry.get::<String>(first, 12),
        Err(HandleError::WrongType)
    );
    assert_eq!(registry.get::<u64>(first, 11), Err(HandleError::WrongType));

    let foreign = NativeHandle::from_parts(8, 11, first.slot(), first.generation());
    assert_eq!(
        registry.get::<String>(foreign, 11),
        Err(HandleError::WrongExtension)
    );

    assert_eq!(registry.remove::<String>(first, 11).unwrap(), "viewer");
    assert_eq!(registry.get::<String>(first, 11), Err(HandleError::Stale));
    let reused = registry.insert(11, String::from("next")).unwrap();
    assert_eq!(reused.slot(), first.slot());
    assert_ne!(reused.generation(), first.generation());
}

#[test]
fn exhaustion_finalization_and_deactivation_are_deterministic() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut registry = HandleRegistry::with_limits(9, 2, 1);
    let zero = registry.insert(21, Tracked(0, Rc::clone(&events))).unwrap();
    let one = registry.insert(21, Tracked(1, Rc::clone(&events))).unwrap();
    assert_eq!(
        registry.insert(21, Tracked(2, Rc::clone(&events))),
        Err(HandleError::Exhausted)
    );
    events.borrow_mut().clear();

    drop(registry.remove::<Tracked>(zero, 21).unwrap());
    assert_eq!(&*events.borrow(), &[0]);
    assert_eq!(
        registry.insert(21, Tracked(3, Rc::clone(&events))),
        Err(HandleError::Exhausted)
    );
    events.borrow_mut().clear();

    registry.deactivate();
    assert_eq!(&*events.borrow(), &[1]);
    assert!(matches!(
        registry.get::<Tracked>(one, 21),
        Err(HandleError::Inactive)
    ));
    assert_eq!(
        registry.insert(21, Tracked(4, Rc::clone(&events))),
        Err(HandleError::Inactive)
    );
    events.borrow_mut().clear();

    let mut ordered = HandleRegistry::with_limits(10, 2, u32::MAX);
    ordered.insert(21, Tracked(5, Rc::clone(&events))).unwrap();
    ordered.insert(21, Tracked(6, Rc::clone(&events))).unwrap();
    ordered.deactivate();
    assert_eq!(&*events.borrow(), &[5, 6]);
}

#[derive(Debug)]
struct Tracked(u8, Rc<RefCell<Vec<u8>>>);

impl Drop for Tracked {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}
