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

			table.insert(one_piece, {
				name = value.name,
				size = value.size,
				data = string.unpack(value.fmt, part_bytes)
			})

			index = next_index
		end
		table.insert(new_list, one_piece)
	end
end