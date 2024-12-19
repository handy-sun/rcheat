-- require 'core'

---@class Structure
---@field match_table table which use regex to get alias name
Structure = {}
Structure.__index = Structure
Structure.match_table = {
	['pcmStateList'] = 'psl',
}

function Structure:new_psl(bytes)
	self.psl_col = {
		{ name = 'id', size = 4, fmt = 'i' },
		{ name = 'stared', size = 1, fmt = 'I' },
		{ name = 'act', size = 4, fmt = 'f' },
	}

	return setmetatable({ psl = SetupTableData(bytes, self.psl_col) }, Structure)
end
