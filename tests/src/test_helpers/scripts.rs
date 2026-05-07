use crate::{
    metadata_builders::{script_hash, DeployedScript},
    Loader,
};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::ScriptHashType},
    context::Context,
};

pub fn deploy_data2_script(
    context: &mut Context,
    binary_name: &str,
    args: Bytes,
) -> DeployedScript {
    deploy_script(context, binary_name, ScriptHashType::Data2, args)
}

pub fn deploy_data_script(context: &mut Context, binary_name: &str, args: Bytes) -> DeployedScript {
    deploy_script(context, binary_name, ScriptHashType::Data, args)
}

pub fn deploy_script(
    context: &mut Context,
    binary_name: &str,
    hash_type: ScriptHashType,
    args: Bytes,
) -> DeployedScript {
    let out_point = context.deploy_cell(Loader::default().load_binary(binary_name));
    let script = context
        .build_script_with_hash_type(&out_point, hash_type, args)
        .expect("build deployed script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn always_success_lock(context: &mut Context, args: Bytes) -> DeployedScript {
    always_success_lock_with_hash_type(context, ScriptHashType::Data2, args)
}

pub fn always_success_lock_with_hash_type(
    context: &mut Context,
    hash_type: ScriptHashType,
    args: Bytes,
) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(&out_point, hash_type, args)
        .expect("build always-success lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn always_success_lock_empty(context: &mut Context) -> DeployedScript {
    always_success_lock(context, Bytes::new())
}

pub fn non_whitelisted_lock(context: &mut Context) -> DeployedScript {
    let out_point = context.deploy_cell(Bytes::from(vec![1u8]));
    let script = context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data2, Bytes::new())
        .expect("build non-whitelisted lock");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn fake_data2_script(context: &mut Context, args_hash: [u8; 32]) -> DeployedScript {
    let out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let script = context
        .build_script_with_hash_type(
            &out_point,
            ScriptHashType::Data2,
            Bytes::from(args_hash.to_vec()),
        )
        .expect("build fake Data2 script");
    let script_hash = script_hash(&script);
    DeployedScript {
        out_point,
        script,
        script_hash,
    }
}

pub fn sudt_meta_script(context: &mut Context, args: Bytes) -> DeployedScript {
    deploy_data2_script(context, "sudt-meta", args)
}

pub fn xudt_meta_script(context: &mut Context) -> DeployedScript {
    deploy_data2_script(context, "xudt-meta", Bytes::from(vec![2u8; 32]))
}

pub fn sudt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "sudt", Bytes::from(meta_type_hash.to_vec()))
}

pub fn xudt_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "xudt", Bytes::from(meta_type_hash.to_vec()))
}

pub fn access_list_script(context: &mut Context, meta_type_hash: [u8; 32]) -> DeployedScript {
    deploy_data2_script(context, "access-list", Bytes::from(meta_type_hash.to_vec()))
}
