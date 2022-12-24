use std::cell::RefCell;
use std::ops::Deref;
use std::time::Duration;
use regex::Regex;

use ic_cdk::api::call::{method_name, accept_message,reject_message};
use ic_cdk::timer::TimerId;

const REJECT_MESSAGE_MUST_CALL_INIT_FIRST:&str = "init function must be called before anything else";
const INIT_DOS_TIMER_INTERVAL:u8 = 10;

thread_local! {
    pub static I:RefCell<u8> = RefCell::default();
    static DO_NOT_ALLOW_METHODS:RefCell<Vec<String>> = RefCell::default();
    static DOS_TIMER_INTERVAL:RefCell<u8> = RefCell::new(INIT_DOS_TIMER_INTERVAL);
    static DOS_TIMER:RefCell<TimerId> = RefCell::default();

    static PROPOSALS:RefCell<Vec<Proposal>> = RefCell::default();
}

use crate::DOS_INIT_FUNCTION;
use crate::user_init_function;

#[derive(Debug, Clone,Copy, PartialEq)]
enum ProposalState {
    Created,
    _Voting,
    _Voted,
    _Accepted,
    _Executing,
    Executed,
    _Rejected,
}

#[derive(Debug, Clone)]
struct Proposal {

    uuid: String,
    created_ts: u64,
    last_updated_ts: u64,
    text_proposal:String,
    state: ProposalState,
    proposal_type : String,
    method_name: String,
    proposal_vote_type: String,
}

impl Proposal {
    pub fn new(text_proposal:String, uuid: String, created_ts:u64, last_updated_ts: u64) -> Self{

        // ADD expensive_function 0 0
        // REM expensive_function 0 0

        let text_proposal2 = text_proposal.clone();

        let re = Regex::new(r"(ADD|REM) ([a-zA-Z_/\d]*) (\d*)").unwrap();
        let caps = re.captures(&text_proposal).unwrap();
    
        
        Self {
            uuid,
            created_ts,
            last_updated_ts,
            text_proposal: text_proposal2, 
            state: ProposalState::Created , 
            proposal_type: String::from(&caps[1]), 
            method_name: String::from(&caps[2]), 
            proposal_vote_type: String::from(&caps[3]),
        }
    }
}


impl Proposal {
    pub fn set_state(&mut self, new_state: ProposalState) {
        self.state = new_state;
    }
    pub fn get_state(&self)-> ProposalState {
        self.state
    }

    pub fn get_proposal_type(&self) -> String {
        self.proposal_type.clone()
    }

    pub fn get_method_name(&self) -> String {
        self.method_name.clone()
    }
    pub fn _get_proposal_vote_type (&self) -> String {
        self.proposal_vote_type.clone()
    }
    pub fn get_text_proposal(&self) -> String {
        self.text_proposal.clone()
    }
    pub fn get_uuid(&self) -> String {
        self.uuid.clone()
    }

    pub fn get_last_updated_ts(&self) -> u64 {
        self.last_updated_ts
    }

}
async fn proposal_checker() {
    PROPOSALS.with(|refcell| {
        let proposals = &mut *refcell.borrow_mut();
        for  proposal in proposals {
            if proposal.state == ProposalState::Created {

                if proposal.get_proposal_type().eq_ignore_ascii_case("ADD") {
                    ic_cdk::spawn( add_method_to_do_not_call_methods (proposal.get_method_name()));
                }
                if proposal.get_proposal_type().eq_ignore_ascii_case("REM") {
                    ic_cdk::spawn(remove_method_from_do_not_call_methods(proposal.get_method_name()));
                }
                (*proposal).set_state(ProposalState::Executed);
            }
        }
    });

    let current_ts = ic_cdk::api::time();

    PROPOSALS.with(|refcell| {
        let proposals = &mut *refcell.borrow_mut();
        let mut i = 0;
        'endProposals: loop {
            if proposals.len() < i+1 {
                break 'endProposals;
            }
            
            if proposals.get(i).unwrap().get_state() == ProposalState::Executed {
                if current_ts -  proposals.get(i).unwrap().get_last_updated_ts() > 10000000000 {

                    ic_cdk::println!("Removing proposal : {:?}", proposals.get(i).unwrap());
                    proposals.remove(i);
                    break 'endProposals;
                }
            }
            i = i+1;
        }
    });

}

#[ic_cdk_macros::update]
async fn dos_set_timer_interval(interval: String) -> String {

    let interval = interval.parse::<u64>().unwrap();
    DOS_TIMER.with( |refcell| {
        let timer_id = &mut *refcell.borrow_mut();
        ic_cdk::timer::clear_timer(*timer_id);
 
    });

    DOS_TIMER.with(move |refcell| {
        let timer_id = ic_cdk::timer::set_timer_interval(Duration::from_secs(interval), || ic_cdk::spawn(proposal_checker()));
        refcell.replace(timer_id);
    });

   
    format!("ok")
}

#[ic_cdk_macros::update]
async fn init_function(_name: String) -> String {
    I.with(| refcell|{
        refcell.replace(1);
    });

    DOS_TIMER_INTERVAL.with(|refcell| {
        let timer_id = ic_cdk::timer::set_timer_interval(Duration::from_secs(*refcell.borrow().deref() as u64), || ic_cdk::spawn(proposal_checker()));
        DOS_TIMER.with(move |refcell| {
            refcell.replace(timer_id);
        })
    }); 

    user_init_function(_name).await;
    format!("ok")
}

pub fn guard_query_function() -> Result<(), String> {
    I.with(|refcell|{
        let i = &*refcell.borrow();
        if *i == 0 {
            Err(REJECT_MESSAGE_MUST_CALL_INIT_FIRST.to_owned())
        }
        else {
            // ideally we should go through the do-not-call-list 
            // However in non-replicated query mode, we do not 
            // get access to the method_name. Hence we cannot 
            // 
            Ok(())
        }
    })    
}



#[ic_cdk_macros::inspect_message]
pub async fn inspect_message_function() {

    I.with(|refcell|{
        let i = &*refcell.borrow();

        if *i == 0 {
            if method_name().eq_ignore_ascii_case(DOS_INIT_FUNCTION) {
                accept_message();
            }
            else {
                reject_message();
            }
        }
        else {
            if method_name().eq_ignore_ascii_case(DOS_INIT_FUNCTION) {
                reject_message();
            }
            else {
                DO_NOT_ALLOW_METHODS.with(|do_not_allow_methods|{

                    let do_not_allow_methods = &*do_not_allow_methods.borrow();
                    let mut found = false;
                    for method in do_not_allow_methods {
                        if method.eq_ignore_ascii_case(&method_name()) {
                            found = true;
                            break;
                        }
                    }
                    if found == true {
                        reject_message();
                    }
                });

                accept_message();
            }
        } 

    })    
}


#[ic_cdk_macros::query(guard="guard_query_function")]

async fn dos_list_proposals() -> String {

    PROPOSALS.with(|refcell| {
        let mut  ret_str = String::new();
        let proposals = &*refcell.borrow();

        for proposal in proposals {

            ret_str.push_str(&format!(" ( uuid: {}, state: {:?}, text_proposal: {}, last_updated: {} ) ", 
                proposal.get_uuid(), proposal.get_state(), proposal.get_text_proposal(), proposal.get_last_updated_ts()));
            ic_cdk::println!("{}", ret_str)
        }

        ic_cdk::println!("FINAL -> {}", ret_str);
        ret_str
    })

}

#[ic_cdk_macros::update]
async fn dos_add_proposal(proposal: String) -> String {

    let rbytes = ic_cdk::api::management_canister::main::raw_rand().await.unwrap();

    PROPOSALS.with(move |refcell| {
        let proposals = &mut *refcell.borrow_mut();

        let ts = ic_cdk::api::time();
        let uuid = ic_cdk::export::Principal::from_slice(&rbytes.0[0..28]);
        let uuid = uuid.to_string();


        let proposal = Proposal::new(proposal, uuid, ts, ts);

        proposals.insert(0, proposal);
    });
    format!("ok")
}

#[ic_cdk_macros::query(guard="guard_query_function")]

async fn dos_list_methods () -> String {
    let mut ret_str = String::new();

    DO_NOT_ALLOW_METHODS.with(move |refcell| {
        let methods = &*refcell.borrow();
        for method in methods {
            ret_str.push_str(&format!("({}), ", method));

        }
        ret_str
    })
}


async fn add_method_to_do_not_call_methods (method:String) {

    let element = method.clone();

    DO_NOT_ALLOW_METHODS.with(move |do_not_allow_methods|{
        let do_not_allow_methods = &mut *do_not_allow_methods.borrow_mut();
        (*do_not_allow_methods).insert(0,element);
    });
}


async fn remove_method_from_do_not_call_methods (method:String) {
    
    DO_NOT_ALLOW_METHODS.with(|do_not_allow_methods| {

        let mut pos:i8 = -1;

        let do_not_allow_methods = &mut *do_not_allow_methods.borrow_mut();
        let mut start = 0;
        
        while start < do_not_allow_methods.len() {

            if do_not_allow_methods.get(start).unwrap().eq_ignore_ascii_case(&method) {
                pos = start as i8;
                break;        
            }
            start = start + 1;
        }

        if pos != -1 {
            do_not_allow_methods.remove(pos as usize);
        }

    });

}
