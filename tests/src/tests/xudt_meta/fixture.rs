use super::*;

pub(super) fn with_config_flags(data: Bytes, config_flags: u8) -> Bytes {
    let mut data = data.to_vec();
    let config_offset = u32::from_le_bytes(data[4..8].try_into().expect("config offset")) as usize;
    data[config_offset] = config_flags;
    Bytes::from(data)
}

pub(super) fn malformed_name_meta_data() -> Bytes {
    let name = 1u32.to_le_bytes().to_vec();
    replace_xudt_meta_table_field(xudt_meta_data(0, 0, None, None, None, Vec::new()), 3, &name)
}

pub(super) fn oversized_name_meta_data() -> Bytes {
    let mut name = 1025u32.to_le_bytes().to_vec();
    name.extend_from_slice(&vec![0u8; 1025]);
    replace_xudt_meta_table_field(xudt_meta_data(0, 0, None, None, None, Vec::new()), 3, &name)
}

pub(super) fn with_name(data: Bytes, name: &[u8]) -> Bytes {
    let mut field = (name.len() as u32).to_le_bytes().to_vec();
    field.extend_from_slice(name);
    replace_xudt_meta_table_field(data, 3, &field)
}

pub(super) fn replace_xudt_meta_table_field(
    data: Bytes,
    field_index: usize,
    replacement: &[u8],
) -> Bytes {
    let data = data.to_vec();
    let first_offset = read_u32(&data, 4) as usize;
    let field_count = first_offset / 4 - 1;
    let mut offsets = Vec::with_capacity(field_count + 1);
    for index in 0..field_count {
        offsets.push(read_u32(&data, 4 + index * 4) as usize);
    }
    offsets.push(data.len());

    let old_start = offsets[field_index];
    let old_end = offsets[field_index + 1];
    let delta = replacement.len() as isize - (old_end - old_start) as isize;
    let new_total = (data.len() as isize + delta) as usize;

    let mut result = Vec::with_capacity(new_total);
    result.extend_from_slice(&new_total.to_le_bytes()[..4]);
    for index in 0..field_count {
        let offset = if index <= field_index {
            offsets[index]
        } else {
            (offsets[index] as isize + delta) as usize
        };
        result.extend_from_slice(&(offset as u32).to_le_bytes());
    }
    result.extend_from_slice(&data[first_offset..old_start]);
    result.extend_from_slice(replacement);
    result.extend_from_slice(&data[old_end..]);

    Bytes::from(result)
}

pub(super) fn read_u32(data: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(data[start..start + 4].try_into().expect("u32 field"))
}

pub(super) fn half_domain_shard() -> Bytes {
    custom_shard([0u8; 32], [0x7fu8; 32], Vec::new())
}

pub(super) fn access_list_shard_with_extra_field() -> Bytes {
    let mut data = Vec::new();
    data.extend_from_slice(&84u32.to_le_bytes());
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&80u32.to_le_bytes());
    data.extend_from_slice(&84u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 32]);
    data.extend_from_slice(&[0xffu8; 32]);
    data.extend_from_slice(&0u32.to_le_bytes());
    Bytes::from(data)
}

pub(super) struct UpdateCase {
    pub(super) context: Context,
    pub(super) tx: TransactionView,
}

pub(super) fn create_meta_tx_with_udt_output_data(
    current_supply: u128,
    udt_outputs_data: Vec<Bytes>,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("xudt-meta"));
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = calculate_type_id(&input, 0);
    let meta = {
        let script = context
            .build_script_with_hash_type(
                &meta_out_point,
                ScriptHashType::Data2,
                Bytes::from(type_id.to_vec()),
            )
            .expect("build deployed Data2 meta script");
        let script_hash = script_hash(&script);
        DeployedScript {
            out_point: meta_out_point,
            script,
            script_hash,
        }
    };
    let xudt = xudt_script(&mut context, meta.script_hash);

    let mut outputs = vec![typed_output(&lock.script, &meta.script, 100_000_000_000)];
    let mut outputs_data = vec![xudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        current_supply,
        Some(input_lock_authority(lock.script_hash)),
        None,
        None,
        Vec::new(),
    )];
    for data in udt_outputs_data {
        outputs.push(typed_output(&lock.script, &xudt.script, 100_000_000_000));
        outputs_data.push(data);
    }

    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&xudt))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}

pub(super) fn create_meta_tx_with_access_outputs(
    config_flags: u8,
    include_full_access_list: bool,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta_out_point = context.deploy_cell(Loader::default().load_binary("xudt-meta"));
    let input = create_funding_input(&mut context, &lock.script, 1_000_000_000_000);
    let type_id = calculate_type_id(&input, 0);
    let meta = {
        let script = context
            .build_script_with_hash_type(
                &meta_out_point,
                ScriptHashType::Data2,
                Bytes::from(type_id.to_vec()),
            )
            .expect("build deployed Data2 meta script");
        let script_hash = script_hash(&script);
        DeployedScript {
            out_point: meta_out_point,
            script,
            script_hash,
        }
    };
    let authority = input_lock_authority(lock.script_hash);
    let output_meta_data = xudt_meta_data(
        config_flags,
        0,
        Some(authority.clone()),
        None,
        Some(authority),
        Vec::new(),
    );

    let mut builder = TransactionBuilder::default()
        .input(input)
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta));

    if include_full_access_list {
        let access_list = access_list_script(&mut context, meta.script_hash);
        builder = builder
            .output(typed_output(
                &lock.script,
                &access_list.script,
                100_000_000_000,
            ))
            .output_data(full_domain_shard().pack())
            .cell_dep(cell_dep_for_script(&access_list));
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}

pub(super) fn update_meta_tx<F>(build: F) -> UpdateCase
where
    F: FnOnce(&mut Context, &DeployedScript, &DeployedScript) -> (Bytes, Bytes, Vec<ExtraCell>),
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let (input_meta_data, output_meta_data, extra_cells) = build(&mut context, &lock, &meta);

    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );
    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta));

    for extra in extra_cells {
        match extra {
            ExtraCell::Output {
                lock,
                type_script,
                data,
                cell_dep,
            } => {
                builder = builder
                    .output(typed_output(&lock, &type_script, 100_000_000_000))
                    .output_data(data.pack())
                    .cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::Input {
                previous_output,
                cell_dep,
            } => {
                builder = builder
                    .input(
                        CellInput::new_builder()
                            .previous_output(previous_output)
                            .build(),
                    )
                    .cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::Dep { cell_dep } => {
                builder = builder.cell_dep(cell_dep_for_script(&cell_dep));
            }
            ExtraCell::CellDep { previous_output } => {
                builder = builder.cell_dep(cell_dep(previous_output));
            }
        }
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}

pub(super) fn update_meta_tx_with_duplicate_outputs() -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let meta_data = xudt_meta_data(0, 0, None, None, None, Vec::new());
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        meta_data.clone(),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(meta_data.clone().pack())
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}

pub(super) fn access_mode_transition_tx(
    input_flags: u8,
    output_flags: u8,
    include_full_input: bool,
    include_full_output: bool,
) -> UpdateCase {
    update_meta_tx(|context, lock, meta| {
        let authority = input_lock_authority(lock.script_hash);
        let mut extra_cells = Vec::new();

        if include_full_input {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Input {
                previous_output: create_typed_cell(
                    context,
                    &lock.script,
                    &access_list.script,
                    100_000_000_000,
                    full_domain_shard(),
                ),
                cell_dep: access_list,
            });
        }

        if include_full_output {
            let access_list = access_list_script(context, meta.script_hash);
            extra_cells.push(ExtraCell::Output {
                lock: lock.script.clone(),
                type_script: access_list.script.clone(),
                data: full_domain_shard(),
                cell_dep: access_list,
            });
        }

        (
            xudt_meta_data(
                input_flags,
                0,
                None,
                None,
                Some(authority.clone()),
                Vec::new(),
            ),
            xudt_meta_data(output_flags, 0, None, None, Some(authority), Vec::new()),
            extra_cells,
        )
    })
}

pub(super) fn update_meta_tx_with_udt_delta(
    input_supply: u128,
    output_supply: u128,
    input_udt_amount: Option<u128>,
    output_udt_amount: Option<u128>,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let xudt = xudt_script(&mut context, meta.script_hash);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = xudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        input_supply,
        Some(authority.clone()),
        None,
        None,
        Vec::new(),
    );
    let output_meta_data = xudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        output_supply,
        Some(authority),
        None,
        None,
        Vec::new(),
    );
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );

    let mut builder = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&xudt));

    if let Some(amount) = input_udt_amount {
        let out_point = create_typed_cell(
            &mut context,
            &lock.script,
            &xudt.script,
            100_000_000_000,
            udt_amount_bytes(amount),
        );
        builder = builder.input(CellInput::new_builder().previous_output(out_point).build());
    }

    if let Some(amount) = output_udt_amount {
        builder = builder
            .output(typed_output(&lock.script, &xudt.script, 100_000_000_000))
            .output_data(udt_amount_bytes(amount).pack());
    }

    let tx = context.complete_tx(builder.build());
    UpdateCase { context, tx }
}

pub(super) fn update_meta_tx_with_fake_udt_output(
    input_supply: u128,
    output_supply: u128,
    fake_udt_amount: u128,
) -> UpdateCase {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let meta = meta_script(&mut context);
    let fake_udt = fake_data2_script(&mut context, meta.script_hash);
    let authority = input_lock_authority(lock.script_hash);
    let input_meta_data = xudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        input_supply,
        Some(authority.clone()),
        None,
        None,
        Vec::new(),
    );
    let output_meta_data = xudt_meta_data(
        CONFIG_SUPPLY_TRACKED,
        output_supply,
        Some(authority),
        None,
        None,
        Vec::new(),
    );
    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input_meta_data,
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(&lock.script, &meta.script, 100_000_000_000))
        .output_data(output_meta_data.pack())
        .output(typed_output(
            &lock.script,
            &fake_udt.script,
            100_000_000_000,
        ))
        .output_data(udt_amount_bytes(fake_udt_amount).pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&meta))
        .cell_dep(cell_dep_for_script(&fake_udt))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}

pub(super) fn update_meta_tx_with_output_lock<F>(build_lock: F) -> UpdateCase
where
    F: FnOnce(&mut Context) -> DeployedScript,
{
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let output_lock = build_lock(&mut context);
    let meta = meta_script(&mut context);
    let input = xudt_meta_data(
        0,
        0,
        None,
        Some(input_lock_authority(lock.script_hash)),
        None,
        Vec::new(),
    );
    let output = xudt_meta_data(
        0,
        0,
        None,
        Some(input_lock_authority(lock.script_hash)),
        None,
        Vec::new(),
    );

    let input_out_point = create_typed_cell(
        &mut context,
        &lock.script,
        &meta.script,
        100_000_000_000,
        input,
    );
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_out_point)
                .build(),
        )
        .output(typed_output(
            &output_lock.script,
            &meta.script,
            100_000_000_000,
        ))
        .output_data(output.pack())
        .cell_dep(cell_dep_for_script(&lock))
        .cell_dep(cell_dep_for_script(&output_lock))
        .cell_dep(cell_dep_for_script(&meta))
        .build();
    let tx = context.complete_tx(tx);
    UpdateCase { context, tx }
}

pub(super) enum ExtraCell {
    Output {
        lock: Script,
        type_script: Script,
        data: Bytes,
        cell_dep: DeployedScript,
    },
    Input {
        previous_output: ckb_testtool::ckb_types::packed::OutPoint,
        cell_dep: DeployedScript,
    },
    Dep {
        cell_dep: DeployedScript,
    },
    CellDep {
        previous_output: ckb_testtool::ckb_types::packed::OutPoint,
    },
}
