# Parity with PilonHydraulika

`crates/creabim-services` is a port of the PilonHydraulika calculation core
(C#). To keep the two in step, the networks in
`crates/creabim-services/tests/parity/*.json` are run by both and compared
node by node (flow, size, velocity, losses, critical branch) in
`cargo test -p creabim-services --test parity`.

The C# results (`*.expected.json`) are committed, so the Rust tests need no
.NET. To make them again (after changing the cases or the C# core):

```sh
python3 tools/pilon-parity/gen_cases.py 40      # the networks, from fixed seeds
dotnet run --project tools/pilon-parity -- crates/creabim-services/tests/parity
# the core is taken from ../pilondesigner/core; elsewhere:
dotnet run --project tools/pilon-parity -p:PilonCore=/path/to/PilonHydraulika.Core.csproj -- crates/creabim-services/tests/parity
```

Needs the .NET 10 SDK.
