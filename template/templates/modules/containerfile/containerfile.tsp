import "@typespec/json-schema";
using TypeSpec.JsonSchema;

@jsonSchema("/modules/containerfile.json")
model ContainerfileModule {
    /** The containerfile module is a tool for adding custom Containerfile instructions for custom image builds. 
     * https://blue-build.org/reference/modules/containerfile/
     */
    type: "containerfile";

    /** Lines to directly insert into the generated Containerfile. */
    snippets?: Array<string>;

    /** Names of directories in ./containerfiles/ containing each a Containerfile to insert into the generated Containerfile. */
    containerfiles?: Array<string>;
}