function ParseZedWorkspace {
    $metadata = cargo metadata --no-deps --offline | ConvertFrom-Json
    $env:ZERMINAL_WORKSPACE = $metadata.workspace_root
    $env:RELEASE_VERSION = $metadata.packages | Where-Object { $_.name -eq "zerminal" } | Select-Object -ExpandProperty version
}
