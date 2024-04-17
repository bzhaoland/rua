$(OBJ_DIR)/%.oo: %.cc
	$(COMPILE_CXX_CP_E)

$(OBJ_DIR)/%.ee: %.cc
	$(COMPILE_CXX_CP_E) -E