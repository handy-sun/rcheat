function SetupTableData(bytes, tab_list)
    local index = 1
    local new_list = {}
    while true do
        for _, value in pairs(tab_list) do
        if index + value.size - 1 > #bytes then
            return new_list
        end
        local next_index = index + value.size
        local part_bytes = string.sub(bytes, index, next_index - 1)

        table.insert(new_list, {
            name = value.name,
            size = value.size,
            data = string.unpack(value.fmt, part_bytes)
        })

        index = next_index
        end
    end
end