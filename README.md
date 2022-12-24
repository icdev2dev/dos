
THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHORS DISCLAIM ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT THE AUTHORS BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

# Protecting against a DOS attack
This project provides some capabilities to SmartCanisters for responding to DOS attacks. Specifically this project provides the capabilities to prevent certain update functions from being executed on a dynamic basis in the SmartCanister on the Internet Computer.

Since the Internet Computer has a reverse gas model, it is possible to draining cycles through a focused DOS attack. While this project is not a panacea to the DOS attack in general, it's motivation arises from how to prevent certain particularly expensive functions from being executed (i.e getting into the replicated state) in the case of DOS attack.

## Use Case

### Setup
We have two functions labeled 
- inexpensive function

 The inexpensive function has a cached version of randomness from the InternetComputer. This cache is populated once.

 ```
 "inexpensive_function": (text) -> (text) query ;
 ```


- expensive function

The expensive function calls the internet computer for a live version of randomness every time this function is called.

```
"expensive_function": (text) -> (text);
```

### Required Action -- Normal Ops

Ordinarily we want the both the *expensive* and the *inexpensive* function to be available on the SmartCanister.

```
dfx canister call dos_backend expensive_function ARG
(
  "Hello, ARG. Live Randomness from IC ([151, 194, 156, 74, 50, 223, 136, 28, 235, 35, 167, 156, 210, 226, 142, 68, 86, 181, 90, 87, 10, 112, 181, 182, 19, 120, 13, 105, 36, 145, 131, 31],) !",
)
```

```
dfx canister call dos_backend inexpensive_function ARG
(
  "Hello, ARG. Cached Randomness from IC [236, 89, 167, 28, 244, 36, 54, 93, 138, 160, 164, 136, 156, 168, 85, 62, 173, 97, 179, 146, 65, 99, 90, 133, 95, 82, 238, 106, 124, 177, 49, 52]!",
)
```

### Required Action -- Under DOS

We expect the SmartCanister to reject any calls to *expensive_function*. 

```
dfx canister call dos_backend expensive_function ARG
-- NO Op
```

while the *inexpensive_function* continues to run.
```
dfx canister call dos_backend inexpensive_function ARG
(
  "Hello, ARG. Cached Randomness from IC [236, 89, 167, 28, 244, 36, 54, 93, 138, 160, 164, 136, 156, 168, 85, 62, 173, 97, 179, 146, 65, 99, 90, 133, 95, 82, 238, 106, 124, 177, 49, 52]!",
)
```

### Required Action -- After DOS

We expect ordinary behaviour to resume. 

## HOW

### AT DOS TIME
At the time of DOS attack, you can create a proposal within the SmartCanister to add the expensive functions to a donotcall list. Of course, when you are in the thick of the DOS, this proposal will take some time to percolate through because there are no current *priority queues* in IC.

```
dfx canister call dos_backend dos_add_proposal "ADD expensive_function 0"
```
This proposal would be executed to add the expensive_function to a donotcall list. The implementation relies on inspect_message to reject the message even before entering the replicated state.


### AFTER DOS TIME

After the DOS attack, you create another proposal to remove the expensive function from the donotcall list. 

```
 dfx canister call dos_backend dos_add_proposal "REM expensive_function 0"
```

This removes the expensive function from the donotcall list and functions can resume executing on call.



## Additional Details 

### 0
You MUST invoke the init_function prior to any functions (query or update) from being executed. It in turn provides you to define a user defined init function.

```
pub async fn user_init_function(_arg:String) -> ()
```

The call can only be done once.

```
dfx canister call dos_backend init_function ARG
```

### 1
You can observe executed proposals through
```
 "dos_list_proposals": () -> (text);
(
  " ( uuid: 2gvxd-czth3-5b6vz-udxjd-qvuha-qmocx-v7khc-ggiac-jrfpc-qewap-fa, state: Executed, text_proposal: REM expensive_function 0, last_updated: 1671840361942527789 ) ",
)
```
These proposals disappear after some time to prevent ever expanding list.

### 2
You can see the current methods on the donotcall list.
```
dfx canister call dos_backend dos_list_methods
("(expensive_function), ")
```

## Future Enhancements

### Voting on Proposals
Since a DOS is complicated to detect, the ability to provide multiple views on whether this is an attack or not will be left to Observers. These Observers will provide a kind-of multi-signal detection capability before a a proposal is executed.

### Moving disappearing proposals to icskfs
The upcoming file system (on stable memory) will store the disappearing proposals on to a file for record keeping.