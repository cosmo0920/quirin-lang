import subprocess
for route in ['corpus','system','slice']:
 subprocess.run(['python3','private/w48-cmt-map/attribute_compare.py',route],check=True)
