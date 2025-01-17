use pcsc::*;
use std::error::Error;

/// Clears the NFC tag/card by writing 0x00 to all writable pages.
pub fn clear_nfc_tag() -> Result<(), Box<dyn Error>> {
    let ctx = Context::establish(Scope::User)?;
    println!("PC/SC context established.");
    let mut readers_buf = [0; 2048];
    let mut readers = ctx.list_readers(&mut readers_buf)?;
    let reader = match readers.next() {
        Some(reader) => reader,
        None => {
            println!("No readers are connected.");
            return Ok(());
        }
    };
    println!("Using reader: {:?}", reader);

    let mut card = ctx.connect(reader, ShareMode::Shared, Protocols::ANY)?;
    println!("Card connected.");

    let tx = card.transaction()?;
    println!("Transaction started.");

    for page in 0..=255 {
        // Construct APDU command to write 0x00 to each page
        let write_command = [
            0xFF, // Class
            0xD6, // Instruction (Write Binary)
            0x00, // P1 (Parameter 1, memory address high byte)
            page, // P2 (Parameter 2, memory address low byte, page number)
            0x04, // Lc (Data length to write: 4 bytes per page)
            0x00, 0x00, 0x00, 0x00, // Data to write (4 bytes of 0x00)
        ];

        let mut response_buf = [0; 256];
        let response = tx.transmit(&write_command, &mut response_buf);

        match response {
            Ok(_) => println!("Page {} cleared successfully.", page),
            Err(err) => {
                eprintln!("Failed to clear page {}: {}", page, err);
                break; // Stop further writes on error
            }
        }

        // Optional: Stop early if a terminator (e.g., 0xFE) is detected
        if page >= 20 && response_buf.contains(&0xFE) {
            println!("Detected terminator (0xFE). Stopping early.");
            break;
        }
    }

    match tx.end(Disposition::LeaveCard) {
        Ok(_) => println!("Transaction ended successfully."),
        Err((_, err)) => eprintln!("Failed to end transaction: {}", err),
    }

    match card.disconnect(Disposition::ResetCard) {
        Ok(_) => println!("Card disconnected successfully."),
        Err((_, err)) => eprintln!("Failed to disconnect card: {}", err),
    }

    Ok(())
}
