$(OBJ_DIR):/%.o: %.c
	$(HS_CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c -o $@ $<

$(OBJ_DIR):/%.e: %.c
	$(HS_CC) $(CFLAGS_GLOBAL_CP) $(CFLAGS_LOCAL_CP) -MMD -c -E -o $@ $<