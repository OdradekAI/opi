use crate::ws::pmd::{Decoder, Encoder};
#[test]
fn complete_boundary_controls() {
    for p in [vec![0], vec![1], vec![3, 0, 0], vec![1, 0, 0, 255, 255, 0]] {
        assert_eq!(
            Decoder::new(false).unwrap().decode(&p).unwrap(),
            b"",
            "{p:?}"
        )
    }
    for p in [
        vec![],
        vec![7],
        vec![0, 10, 0, 245, 255, 65],
        vec![0, 0],
        vec![0, 0, 0],
        vec![0, 0, 0, 255],
    ] {
        assert!(Decoder::new(false).unwrap().decode(&p).is_err(), "{p:?}")
    }
}
#[test]
fn rfc_bfinal_hello_and_nonzero_padding() {
    let good = [0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0, 0];
    assert_eq!(
        Decoder::new(false).unwrap().decode(&good).unwrap(),
        b"Hello"
    );
    let mut bad = good;
    bad[6] |= 0x80;
    assert!(Decoder::new(false).unwrap().decode(&bad).is_err());
}
#[test]
fn no_arbitrary_final_stream_count_limit() {
    for n in [1, 2, 99999, 100000, 100001] {
        for tail in [false, true] {
            let mut p = vec![];
            for _ in 0..n {
                p.extend([1, 0, 0, 255, 255])
            }
            if tail {
                p.extend([1, 4, 0, 251, 255]);
                p.extend(b"TAIL")
            }
            p.push(0);
            let mut d = Decoder::new(false).unwrap();
            assert_eq!(
                d.decode(&p).unwrap(),
                if tail { b"TAIL".as_slice() } else { b"" }
            );
            assert_eq!(d.tail_injections, 1);
        }
    }
}
#[test]
fn roundtrip_windows_context_and_large_outputs() {
    for bits in [8, 9, 15] {
        for nct in [false, true] {
            let mut enc = Encoder::new(bits, nct).unwrap();
            let mut dec = Decoder::new(nct).unwrap();
            for input in [
                vec![],
                b"hello".repeat(600),
                vec![b'Z'; 262144],
                b"hello".repeat(600),
            ] {
                let compressed = enc.encode(&input).unwrap();
                assert_eq!(dec.decode(&compressed).unwrap(), input)
            }
            assert_eq!(dec.tail_injections, 4);
        }
    }
}
#[test]
fn invalid_boundary_absorbs_and_releases() {
    let mut d = Decoder::new(false).unwrap();
    assert!(d.decode(&[]).is_err());
    assert!(d.failed());
    assert!(d.decode(&[0]).is_err());
    assert_eq!(d.tail_injections, 0);
}

#[test]
fn explicit_decoded_limits_allow_equality_and_reject_overflow() {
    for limit in [0usize, 1, 16, 1024, 262144] {
        for n in [limit.saturating_sub(1), limit, limit + 1] {
            let input = vec![b'Z'; n];
            let wire = Encoder::new(15, true).unwrap().encode(&input).unwrap();
            let got = Decoder::new(true).unwrap().decode_limit(&wire, limit);
            assert_eq!(got.is_ok(), n <= limit);
            if let Ok(bytes) = got {
                assert_eq!(bytes, input)
            }
        }
    }
}
#[test]
fn invalid_window_admission_is_typed() {
    for bits in [0, 7, 16, 255] {
        assert!(Encoder::new(bits, false).is_err());
        assert!(matches!(
            Decoder::with_window_bits(bits, false),
            Err(crate::ws::pmd::Fault::Window)
        ));
    }
}
