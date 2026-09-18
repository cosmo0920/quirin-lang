import re, sys, collections

path = "measure/gate/tests.log"
target = None
per = collections.OrderedDict()
order = []
strict = 0
loose = 0
for line in open(path, errors="replace"):
    m = re.match(r"^\s+(Running|Doc-tests)\s+(.*)$", line)
    if m:
        target = "%s %s" % (m.group(1), m.group(2).strip())
        order.append(target)
        per[target] = per.get(target, 0)
        continue
    if line.startswith("test result: "):
        strict += 1
        if target is not None:
            per[target] = per.get(target, 0) + 1
    if line.startswith("test result"):
        loose += 1

print("targets:", len(order))
print("strict 'test result: ' lines:", strict)
print("loose  'test result' lines:", loose)
doubled = [(t, n) for t, n in per.items() if n > 1]
print("targets emitting more than one summary:", len(doubled))
for t, n in doubled:
    print("   ", n, t)
zero = [(t, n) for t, n in per.items() if n == 0]
print("targets emitting zero summaries:", len(zero))
for t, n in zero:
    print("   ", t)
