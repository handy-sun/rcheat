---Dump bytes contents to a formated table 
---@param bytes string byte array
---@param tab_list table defined format
---@return table two_dimensional table
function SetupTableData(bytes, tab_list)
	local index = 1
	local new_list = {}
	while true do
		local one_piece = {}
		for _, value in pairs(tab_list) do
			if index + value.size - 1 > #bytes then
				return new_list
			end
			local next_index = index + value.size
			local part_bytes = string.sub(bytes, index, next_index - 1)
			local final_fmt = ''
			if value.fmt == nil then
				final_fmt = string.format('i%d=', value.size)
			else
				final_fmt = string.format('%s%d=', value.fmt, value.size)
			end

			table.insert(one_piece, {
				name = value.name,
				size = value.size,
				data = string.unpack(final_fmt, part_bytes),
			})

			index = next_index
		end
		table.insert(new_list, one_piece)
	end
end

function LoopMatchAlias(origin_text, tab)
	for key, val in pairs(tab) do
		local start_pos, _ = string.find(origin_text, key)
		if start_pos then
			return val
		end
	end
	return ''
end
