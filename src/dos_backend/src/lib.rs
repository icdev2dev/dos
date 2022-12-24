use std::cell::RefCell;

mod dos;
use dos::guard_query_function;
const DOS_INIT_FUNCTION:&str = "init_function";


thread_local! {
    static CRAND:RefCell<Vec<u8>> = RefCell::default();
}

 
pub async fn user_init_function(_arg:String) -> () {
    let _x = ic_cdk::api::management_canister::main::raw_rand().await.ok().unwrap();

    CRAND.with(move |crand| {
        let crand = &mut *crand.borrow_mut();
        crand.resize(32, 0);
        crand.copy_from_slice(&_x.0);
    });
}



#[ic_cdk_macros::update]
async fn expensive_function(name: String) -> String {
    let _x = ic_cdk::api::management_canister::main::raw_rand().await.ok().unwrap();
    format!("Hello,{}. Live Randomness from IC {:?} !", name, _x)
}



#[ic_cdk_macros::query(guard="guard_query_function")]
fn inexpensive_function(name: String) -> String {
    CRAND.with(|crand|{
        let cached_crand = &*crand.borrow();
        format!("Hello, {}. Cached Randomness from IC {:?}!", name, cached_crand)
    })
}


