Import("env")

import os
import sys
from shutil import copyfile

def pre_build(source, target, env):
    if os.path.exists('src/secrets.h'):
        return
    else:
        sys.stderr.write("Warning: Created secrets.h from template!\n")
        copyfile('src/secrets.template.h', 'src/secrets.h')

env.AddPreAction("$BUILD_DIR/src/main.o", pre_build)

