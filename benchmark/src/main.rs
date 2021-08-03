use curl::easy::Easy;
use std::{thread, time::Duration, fmt, sync::{Arc, atomic::{AtomicBool, Ordering}}};

#[cfg(target_family = "unix")]
use rustc_hash::FxHashMap;
#[cfg(target_family = "unix")]
use sysinfo::{System, SystemExt, NetworkExt};

const URL: &str = "CDN_URL_GOES_HERE";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let using_vpn = Arc::new(AtomicBool::new(false));

    let using_vpn_clone = using_vpn.clone(); 

    let join = thread::spawn(move || vpn_check(&using_vpn_clone));

    let cold_cache = make_request()?;
    
    println!(
        "-------------------- Cold Cache --------------------\n{}\n-------------------- Cold Cache --------------------\n", 
        cold_cache
    );

    let mut results_vec: Vec<CurlResult> = vec![];

    let n = 10;

    for _ in 0..n {
        let result = make_request()?;

        results_vec.push(result);
    }

    let mut hot_cache = results_vec.remove(0);

    for result in results_vec {
        hot_cache = hot_cache + result;
    }

    hot_cache.namelookup_time /= n;
    hot_cache.connect_time /= n;
    hot_cache.appconnect_time /= n;
    hot_cache.pretransfer_time /= n;
    hot_cache.redirect_time /= n;
    hot_cache.starttransfer_time /= n;
    hot_cache.total_time /= n;
    
    println!(
        "-------------------- Hot Cache --------------------\n{}\n-------------------- Hot Cache --------------------", 
        hot_cache
    );
    
    let mut minutes: u8 = 0;

    while minutes < 30{
        if using_vpn.load(Ordering::Relaxed) {
            println!("Using VPN. ABORTING MISSION!!");
            return Ok(());
        }
        // Wait for half an hour for cache to get warm
        thread::sleep(Duration::from_secs(60));
        minutes += 1;
    }

    let warm_cache = make_request()?;
    
    println!(
        "-------------------- Warm Cache --------------------\n{}\n-------------------- Warm Cache --------------------", 
        warm_cache
    );

    Ok(())
}

#[cfg(target_family = "unix")]
fn vpn_check(atomic_bool: &AtomicBool) {
    let mut system = System::new();
    let mut track_map: FxHashMap<String, (u64, u64)> = FxHashMap::default();
    loop {
        if atomic_bool.load(Ordering::Relaxed) {
            println!("VPN Detected.");
            break;
        }
        thread::sleep(Duration::from_millis(500));
        system.refresh_networks();
        for (interface, network) in system.networks() {
            if !interface.contains("tun") {
                continue;
            }

            let (total_received, total_transmitted) = match track_map.get(interface) {
                Some(tuple) => *tuple,
                None => {
                    track_map.insert(interface.to_owned(), (network.total_received(), network.total_transmitted()));
                    continue;
                }
            };

            if total_received != network.total_received() || total_transmitted != network.total_transmitted() {
                atomic_bool.store(true, Ordering::Relaxed);
                break;
            }

        }

    }
}

#[cfg(target_family = "windows")]
fn vpn_check(atomic_bool: &AtomicBool){
    for adapter in ipconfig::get_adapters().expect("Couldn't get adapters") {
        if adapter.if_type() == ipconfig::IfType::Unsupported || adapter.if_type() == ipconfig::IfType::Ppp{
            if adapter.oper_status() == ipconfig::OperStatus::IfOperStatusUp {
                println!("-------------- VPN Detected --------------");
                atomic_bool.store(true, Ordering::Relaxed);
                break;
            }
        }
    }
}

struct CurlResult {
    pub headers: String,
    pub namelookup_time: Duration,
    pub connect_time: Duration,
    pub appconnect_time: Duration,
    pub pretransfer_time: Duration,
    pub redirect_time: Duration,
    pub starttransfer_time: Duration,
    pub total_time: Duration,
    pub bytes: Vec<u8>
}

impl std::ops::Add for CurlResult {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            headers: self.headers,
            namelookup_time: self.namelookup_time + other.namelookup_time,
            connect_time: self.connect_time + other.connect_time,
            appconnect_time: self.appconnect_time + other.appconnect_time,
            pretransfer_time: self.pretransfer_time + other.pretransfer_time,
            redirect_time: self.redirect_time + other.redirect_time,
            starttransfer_time: self.starttransfer_time + other.starttransfer_time,
            total_time: self.total_time + other.total_time,
            bytes: self.bytes
        }
    }
}
impl fmt::Display for CurlResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\nName Lookup Time: {}ms | {}ns\nConnect Time: {}ms | {}ns\nApp Connect Time: {}ms | {}ns\nPretransfer Time: {}ms | {}ns\nRedirect Time: {}ms | {}ns\nStartTransfer Time: {}ms | {}ns\nDownload Time: {}ms | {}ns\nTotal Time: {}ms | {}ns\nDownloaded: {} bytes",
            self.headers,
            self.namelookup_time.as_millis(),
            self.namelookup_time.as_nanos(),
            self.connect_time.as_millis(),
            self.connect_time.as_nanos(),
            self.appconnect_time.as_millis(),
            self.appconnect_time.as_nanos(),
            self.pretransfer_time.as_millis(),
            self.pretransfer_time.as_nanos(),
            self.redirect_time.as_millis(),
            self.redirect_time.as_nanos(),
            self.starttransfer_time.as_millis(),
            self.starttransfer_time.as_nanos(),
            self.total_time.as_millis(),
            self.total_time.as_nanos(),
            self.total_time.as_millis(),
            self.total_time.as_nanos(),
            self.bytes.len()
                )
    } 
}

fn make_request() -> Result<CurlResult, Box<dyn std::error::Error>> {
    let mut handle = Easy::new();

    let mut buffer: Vec<u8> = vec![];

    let mut headers: Vec<u8> = vec![];

    handle.url(URL)?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buffer.extend_from_slice(data);
            Ok(data.len())
        }).unwrap();

        transfer.header_function(|header_data| {
            headers.extend_from_slice(header_data);
            true
        }).unwrap();
        transfer.perform()?;
    }

    let result = CurlResult{
        headers: String::from_utf8_lossy(&headers).to_string(),
        namelookup_time: handle.namelookup_time()?,
        connect_time: handle.connect_time()?,
        appconnect_time: handle.appconnect_time()?,
        pretransfer_time: handle.pretransfer_time()?,
        redirect_time: handle.redirect_time()?,
        starttransfer_time: handle.starttransfer_time()?,
        total_time: handle.total_time()?,
        bytes: buffer
    };

    Ok(result)

}
