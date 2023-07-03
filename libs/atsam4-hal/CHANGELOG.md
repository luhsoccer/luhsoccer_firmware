# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 (2022-12-06)

### Bug Fixes

 - <csr-id-65b97a3839616b4d02f9e437ae781b08384763ca/> Update -pac and add critical-section feature usage
   - Also need critical-section-single-core feature from cortex-m
   - And must enable critical-section feature to use Peripherals::take() in
     -pac crates
   - Fix clippy warning

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update -pac and add critical-section feature usage ([`65b97a3`](https://github.com/atsam-rs/atsam4-hal/commit/65b97a3839616b4d02f9e437ae781b08384763ca))
</details>

## 0.2.6 (2022-11-29)

### Bug Fixes

 - <csr-id-44dfd271d858715f73c392b821f9d40fdd203f53/> Update atsam*-pc crates
   cargo upgrade (using cargo-upgrades + cargo-edit)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release atsam4-hal v0.2.6 ([`1299dc7`](https://github.com/atsam-rs/atsam4-hal/commit/1299dc7b48f5306c9e040c294b4690a0d5a45ab6))
    - Update atsam*-pc crates ([`44dfd27`](https://github.com/atsam-rs/atsam4-hal/commit/44dfd271d858715f73c392b821f9d40fdd203f53))
</details>

## 0.2.5 (2022-11-29)

### Bug Fixes

<csr-id-0a29442da1e23c04fe945bf644efb8540619e091/>

 - <csr-id-79e79124fc1faf760e34c51b0e26ce57abde7048/> Update GitHub Actions
   - Replace deprecated actions

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 12 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release atsam4-hal v0.2.5 ([`8e148a0`](https://github.com/atsam-rs/atsam4-hal/commit/8e148a0b7b63fcc8bd6e11cd1ee9e2fdafba490e))
    - Release atsam4-hal v0.2.4 ([`2983535`](https://github.com/atsam-rs/atsam4-hal/commit/29835358e67df28c0d4cd649e9303e812319853d))
    - Update GitHub Actions ([`79e7912`](https://github.com/atsam-rs/atsam4-hal/commit/79e79124fc1faf760e34c51b0e26ce57abde7048))
    - udeps remove unused dependencies ([`0a29442`](https://github.com/atsam-rs/atsam4-hal/commit/0a29442da1e23c04fe945bf644efb8540619e091))
</details>

<csr-unknown>
Add udeps, pants, audit, deny GitHub Action checksFix MainClock::RcOscillator4Mhz typo for atsam4n targetsFix clippy warning<csr-unknown/>

## 0.2.4 (2022-11-29)

### Bug Fixes

<csr-id-edbcf58d7c29b4157b030cecd7a3bbad2fb2ab49/>

 - <csr-id-fc3b210e07bb35cd4acfa1bc3667a19f8088cad6/> Update GitHub Actions
   - Replace deprecated actions

<csr-unknown>
<csr-unknown>
Add udeps, pants, audit, deny GitHub Action checksFix MainClock::RcOscillator4Mhz typo for atsam4n targetsFix clippy warning<csr-unknown>
 udeps remove unused dependencies<csr-unknown/>
<csr-unknown/>
<csr-unknown/>

## 0.2.3 (2022-11-17)

<csr-id-3fc93f3b35c036146a910e5802f85b6df59882df/>
<csr-id-196a4a11fb71788650f295473a05eb0cf0242110/>

### Other

 - <csr-id-3fc93f3b35c036146a910e5802f85b6df59882df/> Refactoring GPIO and added StaticMemoryController prototype.
 - <csr-id-196a4a11fb71788650f295473a05eb0cf0242110/> First partially working gpio.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 124 commits contributed to the release over the course of 848 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release atsam4-hal v0.2.3 ([`540f56f`](https://github.com/atsam-rs/atsam4-hal/commit/540f56fcfd6dc96b8b08111b94ce1e12eef7b6a5))
    - Release atsam4-hal v0.2.2 ([`4257688`](https://github.com/atsam-rs/atsam4-hal/commit/425768807b13243c7e19d907e8bb785b0a77641c))
    - Add generated CHANGELOG.md ([`cd5d187`](https://github.com/atsam-rs/atsam4-hal/commit/cd5d18788ab3a7332f8fc404a63d7e8b862b3ba4))
    - Allow conversion of clock without ownership change ([`414e063`](https://github.com/atsam-rs/atsam4-hal/commit/414e063119c8c4671b0661e8c6a5f2ca0bd3b120))
    - Increment to v0.2.2 ([`bafdf14`](https://github.com/atsam-rs/atsam4-hal/commit/bafdf14f49d4402207fe7ef93b57c4560da2bb35))
    - Small fixes and debugging for spi ([`dd27c96`](https://github.com/atsam-rs/atsam4-hal/commit/dd27c96e1b395fc05372ae1f2b4aa32b735a6795))
    - Update usb-device to 0.2.9 ([`a86785a`](https://github.com/atsam-rs/atsam4-hal/commit/a86785a681a50501f3d402bed4b1eef0fe8af221))
    - [TC] Fix clock enable for channels other than 0 ([`caae5ec`](https://github.com/atsam-rs/atsam4-hal/commit/caae5ec819f6db1bcd1a34d6b683fadc8ba11861))
    - Replace embedded-time with fugit ([`2b46f90`](https://github.com/atsam-rs/atsam4-hal/commit/2b46f90f90f0a3f50412f2e04966098a05cea251))
    - Fix udp documentation warnings ([`1ac0136`](https://github.com/atsam-rs/atsam4-hal/commit/1ac0136e9cdb11e5a28e0abfccf8b213c52fbc36))
    - Add crates.io badges to README.md ([`1ea26de`](https://github.com/atsam-rs/atsam4-hal/commit/1ea26de7b5d9a8c06431158251303a2f706b68e5))
    - Typo in usb-device ([`baf1cf5`](https://github.com/atsam-rs/atsam4-hal/commit/baf1cf59baa29ea89d54620394aec8f00f1e86f7))
    - Merge pull request #59 from haata/master ([`086f5ee`](https://github.com/atsam-rs/atsam4-hal/commit/086f5ee416249762e45be3b5811fdefea2767100))
    - Updating dependencies and adding DwtTimer ([`5ad6303`](https://github.com/atsam-rs/atsam4-hal/commit/5ad63036ac903fd5986cefc72712385b4bd61229))
    - USB 2.0 compliant remote wakeup ([`37de92b`](https://github.com/atsam-rs/atsam4-hal/commit/37de92badb2453eed2c08784c8d5e5b09beba232))
    - defmt debugging improvments ([`0f579e1`](https://github.com/atsam-rs/atsam4-hal/commit/0f579e19156d0b2af5a9097c819ff1a835316a29))
    - Add more defmt support for enums ([`c7e7b22`](https://github.com/atsam-rs/atsam4-hal/commit/c7e7b22f2992d79bd53166bc46ebbcfce9c1778a))
    - Replacing iap_function with C RAM functions ([`d3a8697`](https://github.com/atsam-rs/atsam4-hal/commit/d3a8697259940982ec58fc28a214c1709068e4ed))
    - Updating defmt to 0.3 ([`2847826`](https://github.com/atsam-rs/atsam4-hal/commit/284782616817af6fa4ffb8d0b920e5b078c812b4))
    - Adding SPI Master support ([`6234d32`](https://github.com/atsam-rs/atsam4-hal/commit/6234d32faf1cf052272645f70cc9c9531287fed2))
    - Clippy fix ([`109262d`](https://github.com/atsam-rs/atsam4-hal/commit/109262d0f9052ba95dd1949fe236c374e6c403cd))
    - Adding support for embedded_hal IoPin trait ([`e46fba9`](https://github.com/atsam-rs/atsam4-hal/commit/e46fba914a9585e2dd9fc8c5a02a675f072af94e))
    - Updating to -pac 0.2.0 ([`b660da8`](https://github.com/atsam-rs/atsam4-hal/commit/b660da8c4cf123d3db47f992a7ff7d02fb6239cb))
    - EFC/EEFC support ([`ef0512b`](https://github.com/atsam-rs/atsam4-hal/commit/ef0512b228a4c5a7d240e5fd92d270b07d8e57ec))
    - Fixing UDP atsam4s PLLB clock ([`4d703c7`](https://github.com/atsam-rs/atsam4-hal/commit/4d703c77d48a0b8c59c243f3e25860f1b77c86c0))
    - Adding read_paused PDC trait function ([`2d5fb9d`](https://github.com/atsam-rs/atsam4-hal/commit/2d5fb9d03aa17299e1ad45572e5879e15f509813))
    - Adds support for TC (Timer/Channel Module) ([`e7a1fad`](https://github.com/atsam-rs/atsam4-hal/commit/e7a1fade002213d895ee002491ca1f99ec49886d))
    - Adding support for downgrading to generic gpio pins ([`ea4a384`](https://github.com/atsam-rs/atsam4-hal/commit/ea4a38495adcfb9ecf04ae59cd2db5ce147825df))
    - ADC Support for ATSAM4S ([`99b29d8`](https://github.com/atsam-rs/atsam4-hal/commit/99b29d82ee4c60f2a419cd6828a790307fe8b082))
    - Adding support for ExtFn gpio pins ([`1dad649`](https://github.com/atsam-rs/atsam4-hal/commit/1dad649cfea139814dabaa85f9f04c36ae57df16))
    - Adds support for USB remote wakeup ([`cc26f05`](https://github.com/atsam-rs/atsam4-hal/commit/cc26f05112b6051a8d93c3a2eda50c0468a7065e))
    - Update Cargo.toml ([`5219311`](https://github.com/atsam-rs/atsam4-hal/commit/521931177c743bdfb9bc6ddc3ffc7b24e571b71a))
    - Fixing atsam4n GitHub Actions ([`f21b37a`](https://github.com/atsam-rs/atsam4-hal/commit/f21b37acf774e449059799b61a6bbaaf01968996))
    - USB (UDP) Support for atsam4s and atsam4e ([`080a10a`](https://github.com/atsam-rs/atsam4-hal/commit/080a10a17faa2a1bdbd41aad2c078ec9e3bcebde))
    - Adding System Function control to I/O pins ([`10632f6`](https://github.com/atsam-rs/atsam4-hal/commit/10632f6a82f03b990b873266cdf086fd5bfe8ce5))
    - Update README.md ([`6bae12b`](https://github.com/atsam-rs/atsam4-hal/commit/6bae12b9b23780ee75539e17a1485dfaa957b4ef))
    - Fixing InputPin and RTT ([`b9b88cf`](https://github.com/atsam-rs/atsam4-hal/commit/b9b88cffa068070a39a27f464a5d7d47acc6c40e))
    - Change to expose embedded_time to clients. ([`cdd4c3c`](https://github.com/atsam-rs/atsam4-hal/commit/cdd4c3ce3fed415a9dd200494f674b6bf54089a9))
    - RustFmt fixes. ([`ede7c82`](https://github.com/atsam-rs/atsam4-hal/commit/ede7c8262b3da4a434caa3480cbfa657482ba095))
    - Disable usage of the 4Mhz RC Oscillator with the PLL since it's not supported on the SAM4N. ([`8773c16`](https://github.com/atsam-rs/atsam4-hal/commit/8773c16b177fe82834693ac06491f9817282f774))
    - More clock updates. ([`d673f15`](https://github.com/atsam-rs/atsam4-hal/commit/d673f1546197fe2afa3e3f14ab7b24f8af0b6b58))
    - Clock updates ([`6579746`](https://github.com/atsam-rs/atsam4-hal/commit/65797468ee73e7deaf09f65602c200609022a25c))
    - Rustfmt fixes ([`03273d4`](https://github.com/atsam-rs/atsam4-hal/commit/03273d4ddf1b60cfea33314217608bf7a2940a88))
    - ATSAM4N support ([`ca19889`](https://github.com/atsam-rs/atsam4-hal/commit/ca19889f1cca8901112b9e99f5d03313ad6fd163))
    - Update Cargo.toml ([`3adfe68`](https://github.com/atsam-rs/atsam4-hal/commit/3adfe6810a7317d53a65ab512b512faf8d278dd9))
    - Added missing AtSam4s2 and AtSam4s4 model identifiers. ([`35f7607`](https://github.com/atsam-rs/atsam4-hal/commit/35f7607e23e683e8a19abfdd1b0dfe3e3fd95c34))
    - Removed get_ from getter methods to be more idiomatic. ([`ee344f5`](https://github.com/atsam-rs/atsam4-hal/commit/ee344f50d3f76c47b9b77786e1256913ffc714cc))
    - Small cleanup: * Fixed comment spelling * Fixed TODO in watchdog for SAM4E ([`6fbe0d0`](https://github.com/atsam-rs/atsam4-hal/commit/6fbe0d0056e2c90b0ac0e3d24fa69187efce40c3))
    - Whitespace cleanup ([`9d4c2a1`](https://github.com/atsam-rs/atsam4-hal/commit/9d4c2a1f46e522ef8627677c9585e4fe818f7e5a))
    - Made ChipId structure invariant. ([`e35841e`](https://github.com/atsam-rs/atsam4-hal/commit/e35841eec30cfb1ab8cbcf244593e0ca2a86acc3))
    - Modified decoder to determine chip family and model directly from the register values. ([`aa43ffb`](https://github.com/atsam-rs/atsam4-hal/commit/aa43ffb3de0bba810fd24316c94433b4e5c5d873))
    - Bumped version. ([`68aea8d`](https://github.com/atsam-rs/atsam4-hal/commit/68aea8dc3cbe8c0fba93b113abd9dee5a5294954))
    - Added missing SAM4SD variants to the architecture decoder. ([`302679d`](https://github.com/atsam-rs/atsam4-hal/commit/302679d7e24c13b998339634dbd686f1811c25d6))
    - Removed unused txbufdescblock.rs ([`3271da8`](https://github.com/atsam-rs/atsam4-hal/commit/3271da8396a9558d2d1526ef688a9bee2f28fb3c))
    - Removed unused ci directory ([`da7e6b6`](https://github.com/atsam-rs/atsam4-hal/commit/da7e6b6f9639a90ef2dd9ced5300222c4ef5fc94))
    - Adding cargo doc check to GitHub CI ([`50a6dab`](https://github.com/atsam-rs/atsam4-hal/commit/50a6dab2cb569b9e9993451e589344f45aaf9a27))
    - Update Cargo.toml ([`d510481`](https://github.com/atsam-rs/atsam4-hal/commit/d510481509b57478bac037a39933208c517e58bb))
    - Cargo fmt fixes. ([`aa3e326`](https://github.com/atsam-rs/atsam4-hal/commit/aa3e3261b8e6862dd6961f5a34c4fe7e693ad77d))
    - Added support for CHIPID ([`c76e1c6`](https://github.com/atsam-rs/atsam4-hal/commit/c76e1c676cb4f6655c879d0d69c26f3866fabd36))
    - Adding support for RTT (Real-time Timer) ([`811cd4d`](https://github.com/atsam-rs/atsam4-hal/commit/811cd4d5a74dea297c306d3bb4eedc8354ee4a3c))
    - Replaced local time.rs with the embedded-time crate. ([`efbdb79`](https://github.com/atsam-rs/atsam4-hal/commit/efbdb795a004c2f5cbcaf07f2e66485a3095e834))
    - Update README.md ([`323abd1`](https://github.com/atsam-rs/atsam4-hal/commit/323abd1fd529676e0c6a365bd650471b249fa818))
    - * Removed travisci integration. ([`92d703c`](https://github.com/atsam-rs/atsam4-hal/commit/92d703c6a206470db689c2cc6588607715d43887))
    - Add missing features in lib.rs for new pacs ([`47cfa5a`](https://github.com/atsam-rs/atsam4-hal/commit/47cfa5ae092460ab736f7c5dfe77118551482bac))
    - Adding atsam4e_c and atsam4e_e feature flags ([`d0da322`](https://github.com/atsam-rs/atsam4-hal/commit/d0da3228a90fe2154894a8f3b6e023f44f6ec9d6))
    - Updating to new pacs and including all atsam4e and atsam4s in CI ([`9b01fd3`](https://github.com/atsam-rs/atsam4-hal/commit/9b01fd314ffbb770a10ae78388c1f140cb8e1087))
    - Update Cargo.toml ([`f2ccb00`](https://github.com/atsam-rs/atsam4-hal/commit/f2ccb00c199bc700d0e33cfee9df421478e70831))
    - Updated formatting. ([`e5b29c6`](https://github.com/atsam-rs/atsam4-hal/commit/e5b29c6e64b80eacd0a56789d3f94e60d34f1ce5))
    - Added pub use on embedded_hal::watchdog traits so clients can use enable/disable on watchdog without having to use the embedded_hal. ([`fffc23c`](https://github.com/atsam-rs/atsam4-hal/commit/fffc23c3bd8328604295563a45361fa6cb3d162c))
    - Removed incomplete ethernet controller source.   Will re-add when completed and stable. ([`b3e56d7`](https://github.com/atsam-rs/atsam4-hal/commit/b3e56d7af820dd89e62e82f8de153f7197e9b7aa))
    - Merge branch 'master' of github.com:atsam4-rs/atsam4-hal ([`18e2d34`](https://github.com/atsam-rs/atsam4-hal/commit/18e2d34cd43e641e517d279954887cd89c0de429))
    - Moved eui48 (macaddress) support local. ([`ffbf52b`](https://github.com/atsam-rs/atsam4-hal/commit/ffbf52b1dcc886005d5956ae7c81459d2ba41d4d))
    - Small spelling fix ([`5f1bef6`](https://github.com/atsam-rs/atsam4-hal/commit/5f1bef61ebcaddea9bdc2caf3bdd668d2ed1b00b))
    - Adding feature flag for USB clock ([`32df4a3`](https://github.com/atsam-rs/atsam4-hal/commit/32df4a3a97aec8fd77ef4e33d73cb11633ad0436))
    - Clippy fix ([`10dd1b6`](https://github.com/atsam-rs/atsam4-hal/commit/10dd1b614e9153490ee1d542c4d2f9e7278ab888))
    - Changing ClockController to handle Main, Master and Slow Clocks ([`05c3ed5`](https://github.com/atsam-rs/atsam4-hal/commit/05c3ed5e493f9280af953507ab720639f5829a0b))
    - 12 MHz crystal oscillator support ([`0a27cb8`](https://github.com/atsam-rs/atsam4-hal/commit/0a27cb822f278aad35206647c82aacc11fddb7a9))
    - Updated version to 0.1.6 ([`00c554d`](https://github.com/atsam-rs/atsam4-hal/commit/00c554dc75199b165deafa54d854c93ae6e29c56))
    - Bumped atsam4e16e-pac to version 0.1.4 ([`1c63974`](https://github.com/atsam-rs/atsam4-hal/commit/1c63974988a1632dbfa8b48078f9e4e8b75ba416))
    - Adding atsam4s4b and atsam4s8b ([`a137ecf`](https://github.com/atsam-rs/atsam4-hal/commit/a137ecf88a08308b9b9b022456b6c51dafd3657c))
    - Adding badge for Docs.rs link ([`f9934b7`](https://github.com/atsam-rs/atsam4-hal/commit/f9934b72600b717bb8126a84c702214e36190217))
    - Added change to allow clippy::upper_case_acronyms.   Since the PACs generate these (and macros exist that consume those types exist), they're being allowed. ([`f246a9d`](https://github.com/atsam-rs/atsam4-hal/commit/f246a9d10d7f93edeb3d3a92359ca3d837d50001))
    - Adding GitHub Actions ([`f97465b`](https://github.com/atsam-rs/atsam4-hal/commit/f97465bdfc7ebd5ac9188714cf467ed5406ef953))
    - Fixing clippy warnings ([`b7a7137`](https://github.com/atsam-rs/atsam4-hal/commit/b7a71374a13a7fe09984da56954e0d8cc09f3eda))
    - cargo fmt ([`338fd0f`](https://github.com/atsam-rs/atsam4-hal/commit/338fd0f1f60f2dc300960c70884f628b400aa8c8))
    - Modified location of eui48 dependency ([`b389520`](https://github.com/atsam-rs/atsam4-hal/commit/b3895205a917ed4ff8fa8df1951eb2a473f6d990))
    - Unstable ethernet controller driver.   Changes for satisfy clippy. ([`cebdd89`](https://github.com/atsam-rs/atsam4-hal/commit/cebdd89c34bf7068f6814bd7fcf83487f41c9763))
    - Version 0.1.5 ([`0696afc`](https://github.com/atsam-rs/atsam4-hal/commit/0696afc951c1e40f30e784a9be9ec92cf288332a))
    - Added support for lazy_static.  Modified clock code to add PIOE for SAM4E. ([`175be15`](https://github.com/atsam-rs/atsam4-hal/commit/175be150c90cc13be8029b10ec8edb69c23146ec))
    - Updated Travis token ([`6296ae0`](https://github.com/atsam-rs/atsam4-hal/commit/6296ae0093f542c232fc021ebbe9e90283cf4c6f))
    - Added slack notifcation to travis.yml. ([`9a449b5`](https://github.com/atsam-rs/atsam4-hal/commit/9a449b5879389ef6f186d84de690c63d525f9e49))
    - Cleanup inside lib.rs. ([`3a92b68`](https://github.com/atsam-rs/atsam4-hal/commit/3a92b681ff8f041ed21c75fcee0e9f730caebe8f))
    - Added disable watchdog timer feature. ([`317cfe0`](https://github.com/atsam-rs/atsam4-hal/commit/317cfe0925e815072040f96e02f8215074766acd))
    - Added pre_init() code to set up system clocks before main() is called. ([`bdd51a9`](https://github.com/atsam-rs/atsam4-hal/commit/bdd51a9c2e9a86d37f08861a8ac53a34196021fb))
    - Bumped crate version to 0.1.2. ([`917cc79`](https://github.com/atsam-rs/atsam4-hal/commit/917cc792ff8d34aa1b9d881fc8e92af04d079f16))
    - Added Serial Port support (UART0, UART1 only) ([`e33f18c`](https://github.com/atsam-rs/atsam4-hal/commit/e33f18c2f3d8715741d52ffa611907b3c16a4ea2))
    - Skeleton serial support ([`c201199`](https://github.com/atsam-rs/atsam4-hal/commit/c201199b3ac797c110ec9c49e292b6759544dc1e))
    - Bump crate version to 0.1.1 ([`1ddfedf`](https://github.com/atsam-rs/atsam4-hal/commit/1ddfedfc7afbf3c067c7a94da3ff79398b486975))
    - Updated default feature handling. ([`a4fe7ab`](https://github.com/atsam-rs/atsam4-hal/commit/a4fe7abac2113699d8d75097a2d9b3d48f9ece65))
    - Updated travis to properly set features on build targets. ([`a9054bf`](https://github.com/atsam-rs/atsam4-hal/commit/a9054bfd5dd8e785a2d19673711b36aa4cbeafdd))
    - Removed path specifiers in Cargo.toml for PAC dependencies. ([`af4a607`](https://github.com/atsam-rs/atsam4-hal/commit/af4a6070aa3a5c8b4eee4a2d0da7c2ec0901957b))
    - Fixed too many keywords error on publish. ([`393e36e`](https://github.com/atsam-rs/atsam4-hal/commit/393e36e5b3d638588368f6d2231c6e4a8ff14e44))
    - Updated Cargo.toml to point to local versions of PAC along with version number on crates.io ([`963014d`](https://github.com/atsam-rs/atsam4-hal/commit/963014dc0efef2048db709ad74eb56a77d6f065a))
    - Updated readme to point to correct travis URL for build status. ([`5dbea1a`](https://github.com/atsam-rs/atsam4-hal/commit/5dbea1abfd3d9944052ad126d5c329550147731e))
    - Updated atsam4sd32c crate version to 0.1.1. ([`5b54967`](https://github.com/atsam-rs/atsam4-hal/commit/5b54967484d6a8bd9a2a072bb023464d5daddc68))
    - Travis fixes. ([`d2ad710`](https://github.com/atsam-rs/atsam4-hal/commit/d2ad7106ee5cc731aeb9eef736862a4838ee97cc))
    - TravisCI Support: Updated Cargo.toml with default feature.   Added default target type. ([`cd789ac`](https://github.com/atsam-rs/atsam4-hal/commit/cd789acb793ca3203be35ed97b3e33cc058a8bb6))
    - Added travisci support ([`cd68161`](https://github.com/atsam-rs/atsam4-hal/commit/cd68161e48fb35a1467909e83ae8da6740b60946))
    - Merged in changes for SAM4S ([`bbbbcd9`](https://github.com/atsam-rs/atsam4-hal/commit/bbbbcd9a58d6e3ebd87d07e2f0cf4648017293a4))
    - Working StaticMemoryController driver. ([`631db10`](https://github.com/atsam-rs/atsam4-hal/commit/631db10da841081d0060dc0f9984633faa7e5a3d))
    - Refactoring GPIO and added StaticMemoryController prototype. ([`3fc93f3`](https://github.com/atsam-rs/atsam4-hal/commit/3fc93f3b35c036146a910e5802f85b6df59882df))
    - Added define_pins! macro similar in purpose to how it works in the atsamd create. ([`0d199f3`](https://github.com/atsam-rs/atsam4-hal/commit/0d199f3f91c148862e3b3dd4986aeb2c3dd21b75))
    - New merged GPIO implementation. ([`44e8af5`](https://github.com/atsam-rs/atsam4-hal/commit/44e8af527de983d3a2164f8520f8dd21e472f75d))
    - First partially working gpio. ([`196a4a1`](https://github.com/atsam-rs/atsam4-hal/commit/196a4a11fb71788650f295473a05eb0cf0242110))
    - WIP ([`c014589`](https://github.com/atsam-rs/atsam4-hal/commit/c014589e05673708bc4dc83805f85ee6cd4a7021))
    - Updated GPIO based on embedded-hal. ([`54cb90d`](https://github.com/atsam-rs/atsam4-hal/commit/54cb90dd958e067d238844d672ad067b6885a54f))
    - WIP - Removed board specific code to board crate. ([`5fb7702`](https://github.com/atsam-rs/atsam4-hal/commit/5fb77025f98d3d28df789200ca1fc86503cef11f))
    - Updated with first working simply app on SAM4E_Xplained_pro ([`0d184b7`](https://github.com/atsam-rs/atsam4-hal/commit/0d184b7197ef276eb8a51437316d528a04a02fb5))
    - WIP ([`1cd21cf`](https://github.com/atsam-rs/atsam4-hal/commit/1cd21cf96043a5a34f956587ed2651eb4da86937))
    - Updates (not building) ([`a82908b`](https://github.com/atsam-rs/atsam4-hal/commit/a82908b35e9f106852ebab8a452e6874f2f2a2ff))
    - WIP ([`082c8b5`](https://github.com/atsam-rs/atsam4-hal/commit/082c8b54354484c3a51657f6d4a9f51ba17d0651))
    - WIP ([`946ef5d`](https://github.com/atsam-rs/atsam4-hal/commit/946ef5d67ed356ffc708ddf55d47327906583245))
    - WIP ([`92aca1a`](https://github.com/atsam-rs/atsam4-hal/commit/92aca1a6282e58e183b0a9bbfeb7d675780e8fa2))
    - Initial skeleton checkin.  No working code yet. ([`9d5ec8d`](https://github.com/atsam-rs/atsam4-hal/commit/9d5ec8dcd0b1f17b8f860299aa84d835f520a9b3))
</details>

## 0.2.2 (2022-11-17)

<csr-id-3fc93f3b35c036146a910e5802f85b6df59882df/>
<csr-id-196a4a11fb71788650f295473a05eb0cf0242110/>

### Other

 - <csr-id-3fc93f3b35c036146a910e5802f85b6df59882df/> Refactoring GPIO and added StaticMemoryController prototype.
 - <csr-id-196a4a11fb71788650f295473a05eb0cf0242110/> First partially working gpio.

