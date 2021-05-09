# 0.4.1

 - Fixed segmentation fault if mmap don't work and fallback to malloc in that case.

# 0.4.0

 - The asynchronous transfer API's and control has been rewritten. Please use new_control_in/out instead of low level new_control.
 - Memory leaks fixed in asynchronous transfers.
 - Timeout added to bulk block transfers. HGowever please use the asynchronous API's instead see example/stm32.rs how to use the API.

# 0.3.0

 - Change ControlTransfer API and setup an mmap fort control transfers.
 - get_descriptor_string variants now return error on fails.

# 0.2.4

 - Fix some of the unsafe memory leaks
 - Use mmap for control channel.
 - Remove some code better panic that seg fault. See readme for details what need fixes.

