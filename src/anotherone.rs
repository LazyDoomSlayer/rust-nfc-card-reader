use pcsc::*;
use std::error::Error;

pub fn read_nfc() -> Result<(), Box<dyn Error>> {
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

    let ndef_data = read_full_ndef_data(&tx)?;
    println!("Full NDEF data read: {:02X?}", ndef_data);

    parse_ndef_data(&ndef_data);

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

fn read_full_ndef_data(tx: &Transaction) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut ndef_data = Vec::new();
    let mut page = 0;

    loop {
        let mut response_buf = [0; 256];
        let response = tx.transmit(&[0xFF, 0xB0, 0x00, page, 0x04], &mut response_buf)?;
        print_response(&format!("Read page {} response", page), response);

        if response.len() < 4 {
            return Err(format!("Failed to read page {}: insufficient data", page).into());
        }

        // Append the 4 bytes of data from the current page
        ndef_data.extend_from_slice(&response[..4]);

        // Check for NDEF terminator (0xFE)
        if response.contains(&0xFE) {
            println!("Detected NDEF terminator at page {}.", page);
            break;
        }

        // Stop reading on warning (63 00) or other status
        if response.len() >= 2
            && response[response.len() - 2] == 0x63
            && response[response.len() - 1] == 0x00
        {
            println!(
                "Non-critical warning received at page {}. Stopping further reads.",
                page
            );
            break;
        }

        page += 1;

        // Safeguard: Stop after reading 256 pages to avoid infinite loops
        if page > 255 {
            println!("Reached maximum page limit.");
            break;
        }
    }

    Ok(ndef_data)
}

fn print_response(context: &str, response: &[u8]) {
    println!("{}: {:02X?}", context, response);

    if response.len() >= 2 {
        let sw1 = response[response.len() - 2];
        let sw2 = response[response.len() - 1];
        println!("Status words: {:02X} {:02X}", sw1, sw2);
    }
}

fn parse_ndef_data(data: &[u8]) {
    if data.is_empty() || !data.contains(&0x03) {
        println!("Invalid or empty NDEF data.");
        return;
    }

    // Locate the NDEF start (0x03 TLV)
    if let Some(ndef_start) = data.iter().position(|&b| b == 0x03) {
        let ndef_length = data.get(ndef_start + 1).cloned().unwrap_or(0) as usize;
        let ndef_payload = &data[ndef_start + 2..ndef_start + 2 + ndef_length.min(data.len())];

        println!("NDEF Data Found:");
        println!("Length: {}", ndef_length);

        // Parse individual NDEF records
        let mut offset = 0;
        while offset < ndef_payload.len() {
            if let Some((record, record_length)) = parse_ndef_record(&ndef_payload[offset..]) {
                println!("Record: {:?}", record);
                offset += record_length; // Move to the next record
            } else {
                println!("Invalid or incomplete NDEF record at offset {}.", offset);
                break;
            }
        }
    } else {
        println!("No NDEF message found in the data.");
    }
}

fn parse_ndef_record(data: &[u8]) -> Option<(String, usize)> {
    if data.len() < 3 {
        return None; // Not enough data for a valid record
    }

    // Parse NDEF Header
    let tnf = data[0] & 0x07; // Type Name Format
    let is_short_record = data[0] & 0x10 != 0;
    let type_length = data[1] as usize;

    // Parse Payload Length
    let payload_length = if is_short_record {
        data.get(2).cloned().unwrap_or(0) as usize
    } else if data.len() >= 6 {
        let mut len_bytes = [0; 4];
        len_bytes.copy_from_slice(&data[2..6]);
        u32::from_be_bytes(len_bytes) as usize
    } else {
        return None; // Incomplete record
    };

    let header_length = if is_short_record { 3 } else { 6 };
    let record_end = header_length + type_length + payload_length;

    if data.len() < record_end {
        return None; // Incomplete record
    }

    // Extract type and payload
    let record_type = &data[header_length..header_length + type_length];
    let payload = &data[header_length + type_length..record_end];

    // Convert type and payload to strings
    let record_type_str = String::from_utf8_lossy(record_type).to_string();
    let payload_str = String::from_utf8_lossy(payload).to_string();

    println!(
        "TNF: {:#X}, Type: {}, Payload: {}",
        tnf, record_type_str, payload_str
    );

    Some((payload_str, record_end)) // Return the record and its length
}
