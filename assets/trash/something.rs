use log::{error, info};
use pcsc::{Context, Error, Protocols, Scope, ShareMode, Status2, MAX_BUFFER_SIZE};

/// Encodes `device_id` into a simplified NDEF record.
fn create_ndef_record(device_id: &str) -> Vec<u8> {
    let mut ndef = Vec::new();
    // NDEF Message format: [Header | Type Length | Payload Length | Type | Payload]
    // Header: TNF=0x01 (Well-Known Type), SR=1 (Short Record)
    let tnf_byte = 0b00000001; // TNF = Well-Known, SR = true
    let type_length = 0x01; // Type length for 'T' (Text)
    let payload = device_id.as_bytes();
    let payload_length = payload.len() as u8;
    let type_field = b"T"; // Type for Text

    ndef.push(tnf_byte);
    ndef.push(type_length);
    ndef.push(payload_length);
    ndef.extend_from_slice(type_field);
    ndef.extend_from_slice(payload);

    ndef
}

/// Writes the `device_id` to an NFC tag.
fn write_device_id(device_id: &str) -> Result<(), Error> {
    // Step 1: Establish PC/SC Context
    let context = Context::establish(Scope::User)?;

    // Step 2: List NFC Readers
    let mut buffer = [0; MAX_BUFFER_SIZE];
    let readers = context.list_readers(&mut buffer)?;
    if readers.count() == 0 {
        error!("No NFC readers found.");
        return Err(Error::NoReadersAvailable);
    }
    info!("Readers found: {:?}", readers);

    // Step 3: Connect to the first reader
    let reader_name = readers.into_iter().next().unwrap(); // Get the first reader
    let card = context.connect(reader_name, ShareMode::Shared, Protocols::ANY)?;

    // Step 4: Verify tag type and size
    // Use `status2` to get card information, including ATR.
    let status = card.status2()?;
    let atr = status.atr;
    info!("ATR: {:?}", atr);

    // Step 5: Create NDEF Record for `device_id`
    let ndef_message = create_ndef_record(device_id);
    info!("NDEF Message: {:?}", ndef_message);

    // Step 6: Send APDU command to write data
    // Example APDU: [CLA | INS | P1 | P2 | Lc | Data | Le]
    let write_command = [
        0x00,                     // CLA
        0xD6,                     // INS: Write Binary
        0x00,                     // P1
        0x00,                     // P2
        ndef_message.len() as u8, // Lc: Length of data
    ];
    let mut apdu = write_command.to_vec();
    apdu.extend_from_slice(&ndef_message);

    let mut response = [0; MAX_BUFFER_SIZE];
    let response_len = card.transmit(&apdu, &mut response)?;

    let response_slice = &response[0..response_len];
    info!("Response: {:?}", response_slice);

    // Check for successful response
    if response_slice == [0x90, 0x00] {
        info!("Data written successfully!");
        Ok(())
    } else {
        error!("Failed to write data: {:?}", response_slice);
        Err(Error::UnknownError)
    }
}

fn main() {
    env_logger::init(); // Initialize logging
    match write_device_id("12345-device-id") {
        Ok(_) => println!("Write successful!"),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
