require "woollib/common"
require "woollib/Command"

module Wool
  class Users
    class Queued
      mserializable

      getter user_id : Id
      getter command : Command(Users) | Command(Sweater)

      def_equals_and_hash @user_id, @command

      getter id : Id { Id.from_serializable self }

      def initialize(@user_id, @command)
      end
    end
  end
end
