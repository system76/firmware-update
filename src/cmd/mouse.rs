use core::char;
use uefi::status::Result;

use pointer::Pointer;
use proto::Protocol;

pub fn main() -> Result<()> {
    use uefi::pointer::SimplePointerState;
    use uefi::text::TextInputKey;

    let uefi = unsafe { &mut *::UEFI };

    let mut pointers = Pointer::all();

    let mut events = vec![];
    for (i, mut pointer) in pointers.iter_mut().enumerate() {
        (pointer.0.Reset)(pointer.0, false)?;

        println!("Pointer {}: {:X}, {:?}", i, pointer.0.WaitForInput.0, pointer.0.Mode);
        events.push(pointer.0.WaitForInput)
    }

    println!("Keyboard {:X}", uefi.ConsoleIn.WaitForKey.0);
    events.push(uefi.ConsoleIn.WaitForKey);

    loop {
        let mut index = 0;
        (uefi.BootServices.WaitForEvent)(events.len(), events.as_mut_ptr(), &mut index)?;

        println!("Event {:X}", index);

        if let Some(mut pointer) = pointers.get_mut(index) {
            let mut state = SimplePointerState::default();
            (pointer.0.GetState)(pointer.0, &mut state)?;

            println!("{}: {:?}", index, state);
        } else {
            let mut input = TextInputKey::default();

            let _ = (uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut input);

            println!("{}", char::from_u32(input.UnicodeChar as u32).unwrap_or('?'));

            break;
        }
    }

    Ok(())
}
