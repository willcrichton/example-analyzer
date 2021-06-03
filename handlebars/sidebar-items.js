initSidebarItems({"enum":[["Path","Represents the Json path in templates."],["ScopedJson","A JSON wrapper designed for handlebars internal use case"],["TemplateFileError","A combined error type for `TemplateError` and `IOError`"],["TemplateRenderError","A combined error type for `TemplateError`, `IOError` and `RenderError`"]],"fn":[["html_escape","The default escape fn replaces the characters `&\"<>` with the equivalent html / xml entities."],["no_escape","`EscapeFn` that does not change anything. Useful when using in a non-html environment."],["to_json","Convert any serializable data into Serde Json type"]],"macro":[["handlebars_helper","Macro that allows you to quickly define a handlebars helper by passing a name and a closure."]],"mod":[["template",""]],"struct":[["BlockContext","A data structure holds contextual data for current block scope."],["BlockParams","A map holds block parameters. The parameter can be either a value or a reference"],["Context","The context wrap data you render on your templates."],["Decorator","Render-time Decorator data when using in a decorator definition"],["Handlebars","The single entry point of your Handlebars templates"],["Helper","Render-time Helper data when using in a helper definition"],["PathAndJson","Json wrapper that holds the Json value and reference path information"],["RenderContext","The context of a render call"],["RenderError","Error when rendering data on template."],["TemplateError","Error on parsing template."]],"trait":[["DecoratorDef","Decorator Definition"],["Evaluable","Evaluate decorator"],["HelperDef","Helper Definition"],["JsonRender","Render Json data with default format"],["Output","The Output API."],["Renderable","Render trait"]],"type":[["EscapeFn","This type represents an escape fn, that is a function whose purpose it is to escape potentially problematic characters in a string."],["HelperResult","A type alias for `Result<(), RenderError>`"]]});