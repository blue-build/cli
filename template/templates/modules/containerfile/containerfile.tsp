import "@typespec/json-schema";
using TypeSpec.JsonSchema;

@jsonSchema("/modules/containerfile-latest.json")
model ContainerfileModuleLatest {
    ...ContainerfileModuleV1;
}

@jsonSchema("/modules/containerfile-v1.json")
model ContainerfileModuleV1 {
    /** The containerfile module is a tool for adding custom Containerfile instructions for custom image builds.
     * https://blue-build.org/reference/modules/containerfile/
     */
    @oneOf
    type: "containerfile" | "containerfile@latest" | "containerfile@v1";

    /** Lines to directly insert into the generated Containerfile. */
    snippets?: Array<string>;

    /** Names of directories in ./containerfiles/ containing each a Containerfile to insert into the generated Containerfile. */
    containerfiles?: Array<string>;
}
