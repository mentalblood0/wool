require "woollib/common"
require "woollib/Command"

module Wool
  class User
    mserializable

    enum Role
      User
      Moderator
    end

    class Name
      mserializable

      @@pattern = /\w+(?: ?\w)*/

      getter base : String

      def_equals_and_hash @base

      def initialize(@base)
        after_initialize
      end

      def after_initialize
        raise Exception.new "User base name \"#{@base}\" has invalid pattern, correct pattern is #{@@pattern}" unless @base.match @@pattern
      end
    end

    getter name : Name
    getter role : Role
    getter queue : Array(Command(Sweater) | Command(Users)) = [] of Command(Sweater) | Command(Users)

    def_equals_and_hash @name

    getter id : Id { Id.from_serializable @name }

    def initialize(@name, @role = Role::User, @queue = [] of Command(Sweater) | Command(Users))
    end
  end
end
