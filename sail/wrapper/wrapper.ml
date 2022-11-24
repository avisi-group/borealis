type error = Err_exception of string * string | Err_sail of Reporting.error

let exception_to_result f =
  Printexc.record_backtrace true;
  try Ok (f ()) with
  | Reporting.Fatal_error inner -> Error (Err_sail inner)
  | e -> Error (Err_exception (Printexc.to_string e, Printexc.get_backtrace ()))

let bindings_to_list map = map |> Ast_util.Bindings.to_seq |> List.of_seq

let () =
  Callback.register "internal_util_dedup" (fun a ->
      exception_to_result (fun () -> Util.remove_duplicates a));

  Callback.register "internal_process_file_load_files"
    (fun check options env file_paths ->
      exception_to_result (fun () ->
          let name, ast, env =
            Process_file.load_files ?check options env file_paths
          in
          (name, ast, env)));

  Callback.register "internal_type_check_initial_env" (fun () ->
      exception_to_result (fun () -> Type_check.initial_env));

  Callback.register "internal_bindings_to_list" (fun a ->
      exception_to_result (fun () -> bindings_to_list a));

  Callback.register "internal_bigint_to_string" (fun i ->
      exception_to_result (fun () -> Nat_big_num.to_string i));

  Callback.register "internal_add_num" (fun a b ->
      exception_to_result (fun () ->
          Num.string_of_num
            (Num.add_num (Num.num_of_string a) (Num.num_of_string b))))
