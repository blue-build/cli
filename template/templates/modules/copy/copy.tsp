import "@typespec/json-schema";
using TypeSpec.JsonSchema;

@jsonSchema("/modules/copy-latest.json")
model CopyModuleLatest {
  ...CopyModuleV1;
}

@jsonSchema("/modules/copy-v1.json")
model CopyModuleV1 {
  /** The copy module is a short-hand method of adding a COPY instruction into the Containerfile.
   * https://blue-build.org/reference/modules/copy/
   */
  type: "copy" | "copy@latest" | "copy@v1";

  /** Equivalent to the --from property in a COPY statement, use to specify an image to copy from.
   * By default, the COPY source is the build environment's file tree.
   */
  from?: string;

  /** Path to source file or directory. */
  src: string;

  /** Path to destination file or directory. */
  dest: string;
}
