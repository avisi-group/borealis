open Libsail
open Sail_plugin_isla

type error = Err_exception of string * string | Err_sail of Reporting.error

let exception_to_result f =
  Printexc.record_backtrace true;

  try Ok (f ()) with
  | Reporting.Fatal_error inner ->
      let _ = print_endline "fatal sail error" in
      Error (Err_sail inner)
  | e ->
      let _ = print_endline "exception" in
      Error (Err_exception (Printexc.to_string e, Printexc.get_backtrace ()))

let bindings_to_list map = map |> Ast_util.Bindings.to_seq |> List.of_seq
let list_to_bindings list = list |> List.to_seq |> Ast_util.Bindings.of_seq

let get_lexbuf_string s filename =
  let lexbuf = Lexing.from_string s in
  lexbuf.Lexing.lex_curr_p <-
    {
      Lexing.pos_fname = filename;
      Lexing.pos_lnum = 1;
      Lexing.pos_bol = 0;
      Lexing.pos_cnum = 0;
    };
  lexbuf

let parse_file ?loc:(l = Parse_ast.Unknown) (s : string) (filename : string) :
    Lexer.comment list * Parse_ast.def list =
  try
    let lexbuf = get_lexbuf_string s filename in
    try
      Lexer.comments := [];
      let defs = Parser.file Lexer.token lexbuf in
      (!Lexer.comments, defs)
    with Parser.Error ->
      let pos = Lexing.lexeme_start_p lexbuf in
      let tok = Lexing.lexeme lexbuf in
      raise (Reporting.err_syntax pos ("current token: " ^ tok))
  with Sys_error err -> raise (Reporting.err_general l err)

let () =
  (* Primary functions *)
  Callback.register "parse_file" (fun contents filename ->
      exception_to_result (fun () -> parse_file contents filename));

  Callback.register "preprocess" (fun sail_dir target options file_ast ->
      exception_to_result (fun () ->
          Preprocess.preprocess sail_dir target options file_ast));

  Callback.register "process" (fun defs comments type_env ->
      exception_to_result (fun () ->
          let ast = Parse_ast.Defs defs in
          let ast = Initial_check.process_ast ~generate:true ast in
          let ast = { ast with comments } in

          (* The separate loop measures declarations would be awkward to type check, so
             move them into the definitions beforehand. *)
          let ast = Rewrites.move_loop_measures ast in

          let asserts_termination =
            Option.fold ~none:false ~some:Target.asserts_termination None
          in

          Frontend.check_ast asserts_termination type_env ast));

  Callback.register "descatter" (fun effect_info env ast ->
      exception_to_result (fun () -> Frontend.descatter effect_info env ast));

  Callback.register "type_check_initial_env" (fun () ->
      exception_to_result (fun () -> Type_check.initial_env));

  Callback.register "register_isla_target" (fun () ->
      exception_to_result (fun () ->
          Target.register ~name:"isla" ~options:isla_options
            ~pre_parse_hook:isla_initialize ~rewrites:isla_rewrites isla_target));

  Callback.register "move_loop_measures" (fun ast ->
      exception_to_result (fun () -> Rewrites.move_loop_measures ast));

  Callback.register "target_run_pre_parse_hook" (fun target ->
      exception_to_result (fun () -> Target.run_pre_parse_hook target ()));

  Callback.register "target_asserts_termination" (fun target ->
      exception_to_result (fun () -> Target.asserts_termination target));

  Callback.register "effects_infer_side_effects" (fun asserts_termination ast ->
      exception_to_result (fun () ->
          Effects.infer_side_effects asserts_termination ast));

  Callback.register "target_rewrites" (fun target ->
      exception_to_result (fun () -> Target.rewrites target));

  Callback.register "rewrites_rewrite"
    (fun effect_info env rewrite_sequence ast ->
      exception_to_result (fun () ->
          Rewrites.rewrite effect_info env rewrite_sequence ast));

  Callback.register "generate_jib" (fun ast effect_info env ->
      exception_to_result (fun () ->
          let props = Property.find_properties ast in
          Ast_util.Bindings.bindings props
          |> List.map fst |> Ast_util.IdSet.of_list
          |> Specialize.add_initial_calls;

          (* let ast, env = Specialize.(specialize typ_ord_specialization env ast) in *)
          let cdefs, ctx = jib_of_ast env ast effect_info in
          let cdefs, _ = Jib_optimize.remove_tuples cdefs ctx in
          let cdefs = remove_casts cdefs |> remove_extern_impls |> fix_cons in
          let buf = Buffer.create 256 in
          Jib_ir.Flat_ir_formatter.output_defs buf cdefs;
          Buffer.contents buf));

  (* CLI options *)
  Callback.register "set_non_lexical_flow" (fun b ->
      exception_to_result (fun () -> Nl_flow.opt_nl_flow := b));
  Callback.register "set_no_lexp_bounds_check" (fun b ->
      exception_to_result (fun () -> Type_check.opt_no_lexp_bounds_check := b));

  (* Utility *)
  Callback.register "util_dedup" (fun a ->
      exception_to_result (fun () -> Util.remove_duplicates a));

  Callback.register "bindings_to_list" (fun a ->
      exception_to_result (fun () -> bindings_to_list a));
  Callback.register "list_to_bindings" (fun a ->
      exception_to_result (fun () -> list_to_bindings a));

  Callback.register "effectset_elements" (fun set ->
      exception_to_result (fun () -> Effects.EffectSet.elements set));
  Callback.register "effectset_of_list" (fun list ->
      exception_to_result (fun () -> Effects.EffectSet.of_list list));

  Callback.register "bigint_to_string" (fun i ->
      exception_to_result (fun () -> Nat_big_num.to_string i));
  Callback.register "string_to_bigint" (fun i ->
      exception_to_result (fun () -> Nat_big_num.of_string i));

  Callback.register "add_num" (fun a b ->
      exception_to_result (fun () -> Nat_big_num.add a b))
