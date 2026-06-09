use std::sync::Mutex;

use rand::RngExt;

const MAC_MASK: u64 = 0xfe_ff_ff_ff_ff_ff;
const LOCAL_BIT: u64 = 0x02_00_00_00_00_00;

static RETURNED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

pub struct Mac {
    addr: u64,
}

/// MAC Address pool with automatic reuse
///
/// Forrest needs to assign MAC addresses to virtual machines that are unique
/// at that point in time. We do hower want to reuse the MACs once they are no
/// longer used, to make sure we do not run out of DHCP leases.
pub fn get_mac() -> Mac {
    let addr = RETURNED
        .lock()
        .unwrap()
        .pop()
        .unwrap_or_else(|| rand::rng().random());
    let addr = (addr | LOCAL_BIT) & MAC_MASK;

    Mac { addr }
}

impl std::fmt::Display for Mac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.addr.to_be_bytes();

        for b in &bytes[2..7] {
            write!(f, "{b:02x}:")?;
        }

        write!(f, "{:02x}", bytes[7])
    }
}

impl Drop for Mac {
    fn drop(&mut self) {
        RETURNED.lock().unwrap().push(self.addr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let mac = Mac {
            addr: 0x12_34_56_78_9a_bc,
        };
        let mac_str = mac.to_string();
        assert_eq!(mac_str, "12:34:56:78:9a:bc");
    }

    #[test]
    fn test_local_and_unicast() {
        for _ in 0..1000 {
            let mac_str = get_mac().to_string();
            let nibble_two = mac_str.chars().nth(1).unwrap();

            // Mac sure the second nibble has the first bit unset (unicast) and
            // the second bit set (locally administered).
            assert!(['2', '6', 'a', 'e'].contains(&nibble_two));
        }
    }
}
