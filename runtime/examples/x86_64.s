.global user_main
user_main:
	movl $5, %edi
	call builtin_delay_seg
	ret
