import toml
import os
import shutil

# we start by getting the version we are releaseing
cargo = toml.load("Cargo.toml")
version = cargo['package']['version']

scratch_space = "build_scratch/"
target_dir = "releases/{}".format(version)
# we create the necessary directory releases/$vresion/
try:
    os.makedirs(target_dir)
except:
    pass
try:
    os.makedirs(scratch_space)
except:
    pass

arm = "armv7-unknown-linux-gnueabihf"
intel = "x86_64-unknown-linux-gnu"

pro = "--features=pro" 

for target in [arm, intel]:
    for feature in ["", pro]:
        cmd = "CARGO_TARGET_DIR={} cargo build --release --target {} {}".format(scratch_space, target, feature)
        print("Executing:", cmd)
        os.system(cmd)
        result_file = "{}/{}/release/libredis_sql.so".format(scratch_space, target)
        suffix = "_v{}".format(version)
        if feature == pro:
            suffix += "_PRO"
        if target == arm:
            suffix += "_ARMv7"
        if target == intel:
            suffix += "_x86_64"
        shutil.copyfile(result_file, "{}/redisql{}.so".format(target_dir, suffix))

shutil.rmtree(scratch_space)
