fn finish(mut out: Vec<u32>) -> *mut u32 {
    out[0] = out.len() as u32;
    let mut b = out.into_boxed_slice();
    let p = b.as_mut_ptr();
    std::mem::forget(b);
    p
}

fn failure(e: &str) -> *mut u32 {
    let mut out: Vec<u32> = vec![0, 1];
    let b = e.as_bytes();
    out.push(b.len() as u32);
    out.extend(b.chunks(4).map(|c| {
        let mut w = [0u8; 4];
        w[..c.len()].copy_from_slice(c);
        u32::from_le_bytes(w)
    }));
    finish(out)
}

#[no_mangle]
pub extern "C" fn ktc_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[no_mangle]
pub unsafe extern "C" fn ktc_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub unsafe extern "C" fn ktc_decode(ptr: *const u8, len: usize) -> *mut u32 {
    let data = std::slice::from_raw_parts(ptr, len);
    match crate::decode(data) {
        Ok(a) => {
            let mut out: Vec<u32> = Vec::with_capacity(5 + a.ch.len() * a.len());
            out.extend([0, 0, a.ch.len() as u32, a.sr, a.len() as u32]);
            for c in &a.ch {
                out.extend(c.iter().map(|v| v.to_bits()));
            }
            finish(out)
        }
        Err(e) => failure(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ktc_info(ptr: *const u8, len: usize) -> *mut u32 {
    let data = std::slice::from_raw_parts(ptr, len);
    match crate::info(data) {
        Ok(i) => finish(vec![0, 0, i.chans as u32, i.sr, i.len as u32, i.chunks() as u32]),
        Err(e) => failure(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ktc_decode_chunk(ptr: *const u8, len: usize, c: u32) -> *mut u32 {
    let data = std::slice::from_raw_parts(ptr, len);
    let res = crate::info(data).and_then(|i| crate::decode_chunk(data, &i, c as usize));
    match res {
        Ok((start, pcm)) => {
            let n = pcm.first().map_or(0, |p| p.len());
            let mut out: Vec<u32> = Vec::with_capacity(5 + pcm.len() * n);
            out.extend([0, 0, pcm.len() as u32, start as i32 as u32, n as u32]);
            for p in &pcm {
                out.extend(p.iter().map(|v| v.to_bits()));
            }
            finish(out)
        }
        Err(e) => failure(&e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn ktc_free_result(ptr: *mut u32) {
    let n = *ptr as usize;
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, n)));
}
