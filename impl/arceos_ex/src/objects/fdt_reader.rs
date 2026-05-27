pub fn read_cells(addr: usize, len: usize, cells: usize) -> Option<(u64, usize)> {
    if cells == 0 || cells > 2 {
        return None;
    }
    let bytes = cells.checked_mul(4)?;
    if bytes > len {
        return None;
    }

    let value = if cells == 1 {
        read_be_u32(addr, addr.checked_add(len)?)? as u64
    } else {
        read_be_u64(addr, addr.checked_add(len)?)?
    };
    Some((value, bytes))
}

pub fn read_be_u64(addr: usize, limit: usize) -> Option<u64> {
    let high = read_be_u32(addr, limit)? as u64;
    let low = read_be_u32(addr.checked_add(4)?, limit)? as u64;
    Some((high << 32) | low)
}

pub fn read_be_u32(addr: usize, limit: usize) -> Option<u32> {
    addr.checked_add(4).filter(|end| *end <= limit)?;
    let bytes = [
        read_u8(addr, limit)?,
        read_u8(addr.checked_add(1)?, limit)?,
        read_u8(addr.checked_add(2)?, limit)?,
        read_u8(addr.checked_add(3)?, limit)?,
    ];
    Some(u32::from_be_bytes(bytes))
}

pub fn read_u8(addr: usize, limit: usize) -> Option<u8> {
    if addr >= limit {
        return None;
    }
    Some(unsafe { core::ptr::read_volatile(addr as *const u8) })
}

pub fn cstr_len(start: usize, limit: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor < limit {
        if read_u8(cursor, limit)? == 0 {
            return cursor.checked_sub(start);
        }
        cursor = cursor.checked_add(1)?;
    }
    None
}

pub fn cstr_eq(start: usize, len: usize, expected: &[u8], limit: usize) -> Option<bool> {
    if len != expected.len() {
        return Some(false);
    }
    let mut index = 0;
    while index < expected.len() {
        if read_u8(start.checked_add(index)?, limit)? != expected[index] {
            return Some(false);
        }
        index += 1;
    }
    Some(true)
}

pub fn cstr_starts_with(start: usize, len: usize, expected: &[u8], limit: usize) -> Option<bool> {
    if len < expected.len() {
        return Some(false);
    }
    let mut index = 0;
    while index < expected.len() {
        if read_u8(start.checked_add(index)?, limit)? != expected[index] {
            return Some(false);
        }
        index += 1;
    }
    Some(true)
}

pub fn align4(value: usize) -> Option<usize> {
    value.checked_add(3).map(|value| value & !3)
}
